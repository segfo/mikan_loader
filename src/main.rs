#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(abi_efiapi)]

mod hardware;
mod file;
use core::fmt::Write;
use core::panic::PanicInfo;
#[allow(unused_imports)]
use rlibc;
use uefi::prelude::*;
use crate::file::*;
use crate::hardware::x86_64::io::*;
use uefi::table::boot::{MemoryType, MemoryAttribute};
use uefi::proto::media::file::FileMode;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {unsafe{halt();}}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

struct MemoryMap<'a>{
    buffer:*mut u8,
    buffer_size:usize,
    boot:&'a BootServices,
    map_key:Option<uefi::table::boot::MemoryMapKey>
}

// https://dox.ipxe.org/structEFI__MEMORY__DESCRIPTOR.html
struct MemoryDescriptor{
    phys_start:usize,
    virt_start:usize,
    page_count:u64,
    memory_type:u32,
    memory_attribute:u64
}
impl<'a> MemoryMap<'a>{
    fn new(boot:&'a BootServices)->Self{
        let map_size = boot.memory_map_size();
        MemoryMap{
            buffer:boot.allocate_pool(MemoryType::BOOT_SERVICES_DATA,map_size).unwrap().unwrap(),
            buffer_size:map_size,
            map_key:None,
            boot:boot,
        }
    }
    fn save_memory_map(&mut self,file:&mut Write){
        let mut mmap_buffer = unsafe{core::slice::from_raw_parts_mut(self.buffer,self.buffer_size)};
        let (map_key,iter) = self.boot.memory_map(&mut mmap_buffer).unwrap().unwrap();
        self.map_key = Some(map_key);
        for map in iter{
            let memory_type = match map.ty{
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
                _=>0xffff_ffff
            };
            let map = MemoryDescriptor{
                phys_start:map.phys_start as usize,
                virt_start:map.virt_start as usize,
                page_count:map.page_count as u64,
                memory_type: memory_type,
                memory_attribute: map.att.bits()
            };
            writeln!(file, "{{\"Physical Address\":\"0x{:08x}\",\"Virtual Address\":\"0x{:08x}\",\"Pages\":{},\"Memory Type\":\"0x{:04x}\",\"Attributes\":\"0x{:08x}\"}},",
                        map.phys_start,map.virt_start,map.page_count,map.memory_type,map.memory_attribute);
        }
        writeln!(file, "{{\"Physical Address\":\"0x0\",\"Virtual Address\":\"0x0\",\"Pages\":0,\"Memory Type\":\"0x0\",\"Attributes\":\"0x0\"}}");
    }
    fn get_key(&self)->Option<uefi::table::boot::MemoryMapKey>{
        self.map_key
    }
}

#[entry]
fn efi_main(handle: Handle, st: SystemTable<Boot>) -> Status {
    let boot = st.boot_services();
    let runtime = st.runtime_services();
    let mut memory_map = MemoryMap::new(boot);

    memory_map.save_memory_map(st.stdout());
    let file_handle = open_file(&handle,&boot,"\\memory_map.csv",FileMode::CreateReadWrite);
    let mut file = FileWriter::new(file_handle);
    writeln!(file,"[");
    memory_map.save_memory_map(&mut file);
    writeln!(file,"]");
    file.flush();
    writeln!(st.stdout(), "ok");
    
    loop {
        unsafe {
            halt();
        }
    }
}
