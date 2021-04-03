use core::fmt::Write;
use uefi::prelude::*;
use uefi::proto::{
    loaded_image::LoadedImage,
    media::{
        fs::SimpleFileSystem,
        file::{File, RegularFile, Directory, FileMode, FileAttribute}
    }
};
use uefi::ResultExt;
pub struct FileWriter{
    file:RegularFile
}

impl FileWriter{
    pub fn new(file:RegularFile)->Self{
        FileWriter{file:file}
    }
    pub fn flush(&mut self){
        self.file.flush();
    }
}

impl core::fmt::Write for FileWriter {
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