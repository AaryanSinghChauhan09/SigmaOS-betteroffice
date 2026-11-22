//! SigmaOS component binary: betteroffice

use sigmaos_betteroffice::Component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Component::new().operate()?;
    println!("sigmaos-betteroffice: ok");
    Ok(())
}
