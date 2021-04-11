#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(abi_efiapi)]

mod file;
use crate::file::*;
use common::hardware::*;
use core::fmt::Write;
use core::panic::PanicInfo;
#[allow(unused_imports)]
use rlibc;
use uefi::prelude::*;
use uefi::proto::media::file::FileMode;
use uefi::table::boot::{AllocateType, MemoryAttribute, MemoryType};
use xmas_elf::ElfFile;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
        unsafe {
            io::halt();
        }
    }
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

struct MemoryMap<'a> {
    buffer: *mut u8,
    buffer_size: usize,
    boot: &'a BootServices,
    map_key: Option<uefi::table::boot::MemoryMapKey>,
}

// https://dox.ipxe.org/structEFI__MEMORY__DESCRIPTOR.html
struct MemoryDescriptor {
    phys_start: usize,
    virt_start: usize,
    page_count: u64,
    memory_type: u32,
    memory_attribute: u64,
}
impl<'a> MemoryMap<'a> {
    fn new(boot: &'a BootServices) -> Self {
        let map_size = boot.memory_map_size();
        MemoryMap {
            buffer: boot
                .allocate_pool(MemoryType::BOOT_SERVICES_DATA, map_size)
                .unwrap_success(),
            buffer_size: map_size,
            map_key: None,
            boot: boot,
        }
    }
    fn save_memory_map(&mut self, file: &mut Write) {
        let mut mmap_buffer =
            unsafe { core::slice::from_raw_parts_mut(self.buffer, self.buffer_size) };
        let (map_key, iter) = self.boot.memory_map(&mut mmap_buffer).unwrap_success();
        self.map_key = Some(map_key);
        for map in iter {
            let memory_type = match map.ty {
                MemoryType::RESERVED => 0,
                MemoryType::LOADER_CODE => 1,
                MemoryType::LOADER_DATA => 2,
                MemoryType::BOOT_SERVICES_CODE => 3,
                MemoryType::BOOT_SERVICES_DATA => 4,
                MemoryType::RUNTIME_SERVICES_CODE => 5,
                MemoryType::RUNTIME_SERVICES_DATA => 6,
                MemoryType::CONVENTIONAL => 7,
                MemoryType::UNUSABLE => 8,
                MemoryType::ACPI_RECLAIM => 9,
                MemoryType::ACPI_NON_VOLATILE => 10,
                MemoryType::MMIO => 11,
                MemoryType::MMIO_PORT_SPACE => 12,
                MemoryType::PAL_CODE => 13,
                MemoryType::PERSISTENT_MEMORY => 14,
                _ => 0xffff_ffff,
            };
            writeln!(file,"\"Physical Address\":\"0x{:08x}\",\"Virtual Address\":\"0x{:08x}\",\"Pages\":{},\"Memory Type\":\"{:?}\",\"Attributes\":\"{:?}\"",map.phys_start,map.virt_start,map.page_count,map.ty,map.att);

            let map = MemoryDescriptor {
                phys_start: map.phys_start as usize,
                virt_start: map.virt_start as usize,
                page_count: map.page_count as u64,
                memory_type: memory_type,
                memory_attribute: map.att.bits(),
            };
            // writeln!(file,
            //     "\"Physical Address\":\"0x{:08x}\",\"Virtual Address\":\"0x{:08x}\",\"Pages\":{},\"Memory Type\":\"0x{:04x}\",\"Attributes\":\"0x{:08x}\"",
            //             map.phys_start,map.virt_start,map.page_count,map.memory_type,map.memory_attribute);
        }
    }
    fn get_key(&self) -> Option<uefi::table::boot::MemoryMapKey> {
        self.map_key
    }
    fn exit_boot_services(self) -> (*mut u8, usize) {
        self.boot.free_pool(self.buffer).unwrap_success();
        (self.buffer, self.buffer_size)
    }
}

use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::{FrameBuffer, PixelFormat};

fn set_graphics_resolution(boot: &BootServices, max_resolution: (usize, usize)) {
    if let Ok(gop) = boot.locate_protocol::<GraphicsOutput>() {
        let gop = gop.expect("Warnings encountered while opening GOP");
        let mut gop = unsafe { &mut *gop.get() };
        // グラフィックはPixelFormatがRBGもしくはBGRかつ解像度がmax_resolutionまでで一番大きいものを選ぶ。
        let resolution = search_graphics_mode(&mut gop, max_resolution);
        set_graphics_mode(&mut gop, resolution);
    }
}

fn get_framebuffer_info(boot: &BootServices) -> FrameBufferConfig {
    if let Ok(gop) = boot.locate_protocol::<GraphicsOutput>() {
        let gop = gop.expect("Warnings encountered while opening GOP");
        let mut gop = unsafe { &mut *gop.get() };

        let info = gop.current_mode_info();
        let (h_res, v_res) = info.resolution();
        let pixels_per_scan_line = info.stride();
        let px_fmt = match info.pixel_format() {
            PixelFormat::Bgr => common::hardware::PixelFormat::BGRReserved8BitParColor,
            PixelFormat::Rgb => common::hardware::PixelFormat::RGBReserved8BitParColor,
            _ => panic!("not support pixel format."),
        };
        let mut fb = gop.frame_buffer();
        FrameBufferConfig::new(fb.as_mut_ptr(), pixels_per_scan_line, h_res, v_res, px_fmt)
    } else {
        // No tests can be run.
        panic!("UEFI Graphics Output Protocol is not supported");
    }
}

// グラフィックモードの探索
fn search_graphics_mode(
    gop: &mut GraphicsOutput,
    limit_resolution: (usize, usize),
) -> (usize, usize) {
    let (w, h) = limit_resolution;
    // 限界値がVGA規格未満だったら、とりあえず限界は1920x1080まで上げておく。
    let limit_resolution = if (w < 640 || h < 480) {
        (1920, 1080)
    } else {
        limit_resolution
    };
    let mut max_resolution = (0, 0);
    for mode in gop.modes() {
        let info = mode.unwrap();
        let pix_fmt = info.info().pixel_format();
        if (pix_fmt != PixelFormat::Bgr && pix_fmt != PixelFormat::Rgb) {
            continue;
        }
        let (w, h) = info.info().resolution();
        let (mw, mh) = max_resolution;
        let (lw, lh) = limit_resolution;
        if w > h && (mw < w && mh < h) {
            max_resolution = (w, h);
        }
        if lw <= w && lh <= h {
            break;
        }
    }
    max_resolution
}
fn set_graphics_mode(gop: &mut GraphicsOutput, resolution: (usize, usize)) {
    let mode = gop
        .modes()
        .map(|mode| mode.expect("Warnings encountered while querying mode"))
        .find(|ref mode| {
            let info = mode.info();
            info.resolution() == resolution
        })
        .unwrap();

    gop.set_mode(&mode)
        .expect_success("Failed to set graphics mode");
}
// 解像度の定数
const VGA: (usize, usize) = (640, 480);
const XGA: (usize, usize) = (1024, 768);
const FHD: (usize, usize) = (1920, 1080);

#[entry]
fn efi_main(handle: Handle, st: SystemTable<Boot>) -> Status {
    let boot = &st.boot_services();
    let runtime = &st.runtime_services();
    let file_handle = open_file(&handle, &boot, "\\mikan_kernel", FileMode::Read);
    let mut kernel_file = FileReaderWriter::new(file_handle);
    // なにもないときのメモリマップ
    let mut memory_map = MemoryMap::new(&boot);
    let file_handle = open_file(
        &handle,
        &boot,
        "\\memory_map.csv",
        FileMode::CreateReadWrite,
    );
    let mut file = FileReaderWriter::new(file_handle);
    memory_map.save_memory_map(&mut file);
    file.flush();
    file.close();
    // フレームバッファの設定とバッファ情報の取得
    set_graphics_resolution(&boot, FHD);
    let mut frame_buffer = get_framebuffer_info(&boot);
    // カーネルのロード
    let kernel_main = load_kernel(boot, kernel_file);
    // メモリマップの取得
    let mut memory_map = MemoryMap::new(&boot);
    let file_handle = open_file(
        &handle,
        &boot,
        "\\memory_map_loaded_kernel.csv",
        FileMode::CreateReadWrite,
    );
    let mut file = FileReaderWriter::new(file_handle);
    memory_map.save_memory_map(&mut file);
    file.flush();
    file.close();
    let (buf, buf_len) = memory_map.exit_boot_services();
    let buf = unsafe { core::slice::from_raw_parts_mut(buf, buf_len) };
    st.exit_boot_services(handle, buf).unwrap_success();
    kernel_main(frame_buffer);
    loop {
        unsafe {
            io::halt();
        }
    }
}

fn load_kernel(
    boot: &BootServices,
    mut kernel_file: FileReaderWriter,
) -> (extern "sysv64" fn(FrameBufferConfig) -> !) {
    let kernel_size = kernel_file.get_size(&boot);
    let kernel_image = boot
        .allocate_pool(MemoryType::LOADER_DATA, kernel_size)
        .unwrap_success();
    let kernel_image_buf =
        unsafe { core::slice::from_raw_parts_mut(kernel_image as *mut u8, kernel_size as usize) };
    kernel_file.read(kernel_image_buf);
    kernel_file.close();
    let elf = ElfFile::new(kernel_image_buf).unwrap();
    let mut first = u64::MAX;
    let mut last = 0u64;
    // プログラムヘッダを走査して、LOADセクションに記述されているロード先（仮想アドレス）の最初と最後のアドレスを取得する。
    for ph in elf.program_iter() {
        if ph.get_type().unwrap() != xmas_elf::program::Type::Load {
            continue;
        }
        let start_vaddr = ph.virtual_addr();
        let end_vaddr = ph.virtual_addr() + ph.mem_size();
        last = if last < end_vaddr { end_vaddr } else { last };
        first = if first > start_vaddr {
            start_vaddr
        } else {
            first
        };
    }
    // カーネルロード先のメモリ確保する。
    let HMA_KERNEL_BASE = 0x100000;
    let n_pages = ((last - HMA_KERNEL_BASE) as usize + 0xfff) / 0x1000;
    let kernel_load_area = boot
        .allocate_pages(
            AllocateType::Address(HMA_KERNEL_BASE as usize),
            MemoryType::LOADER_DATA,
            n_pages,
        )
        .unwrap_success();
    // LOADセクションを実メモリ空間にコピーする。
    for ph in elf.program_iter() {
        if ph.get_type().unwrap() != xmas_elf::program::Type::Load {
            continue;
        }
        ph.file_size();
        unsafe {
            kernel_image
                .offset(ph.offset() as isize)
                .copy_to_nonoverlapping(
                    (kernel_load_area + ph.offset()) as *mut u8,
                    ph.file_size() as usize, // ファイルサイズ分コピーする
                );
            // ファイル上のサイズ < メモリ上のサイズ であることがある（.bss等の場合）ため
            // その場合には、0で初期化しておく処理が以下。
            let mem_size = ph.mem_size();
            let file_size = ph.file_size();
            // 転送先メモリのサイズがファイルサイズ以下だったら何もしない。
            if mem_size <= file_size {
                continue;
            }
            let zero_init_area = mem_size - file_size;
            kernel_image
                .offset((kernel_load_area + ph.offset() + ph.file_size()) as isize)
                .write_bytes(0, zero_init_area as usize);
        }
    }
    // エントリポイントのアドレスをファイルバッファ内のELFヘッダから取得する
    let entry_point = unsafe {
        let entry_point: extern "sysv64" fn(FrameBufferConfig) -> ! =
            core::mem::transmute(*((kernel_image as usize + 0x18) as *const usize));
        entry_point
    };
    boot.free_pool(kernel_image).unwrap_success();
    entry_point
}
