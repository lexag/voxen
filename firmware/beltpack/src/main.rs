use crate::device::Device;

#[cfg(all(feature = "desktop", feature = "embedded"))]
compile_error!("feature \"desktop\" and feature \"embedded\" cannot be enabled at the same time");

mod device;
mod hw_interface;
#[cfg(feature = "desktop")]
mod hw_linux;

fn main() {
    loop {
        let res = Device::try_new();
        match res {
            Ok(mut d) => {
                println!("starting device");
                d.start();
                break;
            }
            Err(e) => {
                println!("{e:?}");
            }
        }
    }
}
