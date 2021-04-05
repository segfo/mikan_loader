use core::fmt::Write;
use uefi::prelude::*;
use uefi::table::boot::MemoryType;
use uefi::proto::media::file::FileInfo;
use uefi::proto::{
    loaded_image::LoadedImage,
    media::{
        fs::SimpleFileSystem,
        file::{File, RegularFile, Directory, FileMode, FileAttribute}
    }
};
use uefi::ResultExt;
pub struct FileReaderWriter{
    file:RegularFile
}

impl FileReaderWriter{
    pub fn new(file:RegularFile)->Self{
        FileReaderWriter{file:file}
    }
    pub fn flush(&mut self){
        self.file.flush();
    }
    pub fn close(mut self){
        self.file.close();
    }
    pub fn write(&mut self,s:&str){
        self.write_str(s).unwrap();
    }
    pub fn read(&mut self,buf:&mut [u8])->usize{
        self.file.read(buf);
        0
    }
    pub fn get_size(&mut self,boot:&uefi::prelude::BootServices)->u64{
        let mut buf = [0x0u8;0x0];
        let size = match self.file.get_info::<FileInfo>(&mut buf){
            Err(e)=>{ 
                let buffer_len = e.data().unwrap();
                let buffer = boot.allocate_pool(
                    MemoryType::BOOT_SERVICES_DATA,
                    buffer_len
                ).unwrap_success();
                let buffer = unsafe{core::slice::from_raw_parts_mut(buffer,buffer_len)};
                match self.file.get_info::<FileInfo>(buffer){
                    Ok(info)=>info.unwrap().file_size(),
                    Err(_)=>panic!("")
                }
            },
            Ok(info)=>info.unwrap().file_size()
        };
        size
    }
}

impl core::fmt::Write for FileReaderWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.file.write((s).as_bytes()).unwrap_success();
        Ok(())
    }
}

pub fn open_file(handle:&Handle,boot: &BootServices,file_path:&str,mode:FileMode)->RegularFile{
    let loaded_image = boot.handle_protocol::<LoadedImage>(*handle).unwrap_success().get();
    let device=unsafe {(*loaded_image).device()};
    let file_system = boot.handle_protocol::<SimpleFileSystem>(device).unwrap_success().get();
    let mut root_dir: Directory = unsafe {(*file_system).open_volume().unwrap_success()};
    let file_handle = root_dir.open(file_path,mode,FileAttribute::empty()).unwrap_success();
    unsafe {RegularFile::new(file_handle)}
}