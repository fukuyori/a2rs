use a2rs::cpu::MemoryBus;
use a2rs::memory::{Memory, AppleModel};

fn main() {
    let mut memory = Memory::new(AppleModel::AppleIIe);
    
    // キーを設定
    memory.set_key(b'A');
    
    println!("After set_key('A'):");
    println!("  $C000 = 0x{:02X}", memory.read(0xC000));
    println!("  $C000 = 0x{:02X}", memory.read(0xC000));
    println!("  $C000 = 0x{:02X}", memory.read(0xC000));
    
    println!("\nReading $C010 to clear strobe:");
    println!("  $C010 = 0x{:02X}", memory.read(0xC010));
    
    println!("\nAfter clearing:");
    println!("  $C000 = 0x{:02X}", memory.read(0xC000));
    println!("  $C000 = 0x{:02X}", memory.read(0xC000));
}
