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
    memory_descriptor:*mut uefi::table::boot::MemoryDescriptor,
    boot:&'a BootServices,
    map_key:Option<uefi::table::boot::MemoryMapKey>
}
impl<'a> MemoryMap<'a>{
    fn new(boot:&'a BootServices)->Self{
        let map_size = boot.memory_map_size();
        MemoryMap{
            buffer:boot.allocate_pool(MemoryType::BOOT_SERVICES_DATA,map_size).unwrap().unwrap(),
            buffer_size:map_size,
            memory_descriptor:core::ptr::null_mut(),
            map_key:None,
            boot:boot,
        }
    }
    fn get_memory_map(&mut self)->&[uefi::table::boot::MemoryDescriptor]{
        let mut mmap_buffer = unsafe{core::slice::from_raw_parts_mut(self.buffer,self.buffer_size)};
        self.map_key = Some(self.boot.memory_map(&mut mmap_buffer).unwrap().unwrap().0);
        unsafe{
            core::slice::from_raw_parts(
                self.buffer as *mut uefi::table::boot::MemoryDescriptor,
                self.buffer_size/core::mem::size_of::<uefi::table::boot::MemoryDescriptor>()
            )
        }
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
    let file_handle = open_file(&handle,&boot,"\\memory_map.csv",FileMode::CreateReadWrite);
    let mut file = FileWriter::new(file_handle);
    writeln!(file,"\"Physical Address\",\"Virtual Address\",\"Pages\",\"Memory Type\",\"Attributes\"");
    for map in memory_map.get_memory_map().into_iter(){
        writeln!(st.stdout(), "{:08x},{:08x},{},{:?},{:?}",map.phys_start,map.virt_start,map.page_count,map.ty,map.att);
        writeln!(file, "{:08x},{:08x},{},{:?},{:?}",map.phys_start,map.virt_start,map.page_count,map.ty,map.att);
    }
    file.flush();
    writeln!(st.stdout(), "ok");
    loop {
        unsafe {
            halt();
        }
    }
}
