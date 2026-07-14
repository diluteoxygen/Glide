use input_hooks::accessibility::are_accessibility_keys_enabled;
use input_hooks::InputHook;

fn main() {
    println!("Testing accessibility keys state...");
    let enabled = are_accessibility_keys_enabled();
    println!("Sticky Keys / Filter Keys enabled: {}", enabled);
    
    println!("Starting input hooks...");
    let rx = InputHook::start();
    
    println!("Listening for events. Press Ctrl+C to exit.");
    for event in rx {
        println!("Event: {:?}", event);
    }
}
