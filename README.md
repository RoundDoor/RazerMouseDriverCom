# RazerMouseDriverCom Library

This library provides modules and functionality for interacting with Razer mouse devices.

## Modules

- `nt`: Contains low-level utilities and interactions.
- `rzctl`: Provides control functions for Razer mouse devices.

## Example Test

Below is an example test function that demonstrates how to initialize the device and move the mouse:

```rust
#[cfg(test)]
mod tests {
    use super::rzctl;

    #[test]
    fn test_mouse_move() {
        // Initialize the device
        if rzctl::init() {
            // Move the mouse to (100, 100) from the current position
            rzctl::mouse_move(100, 100, true);
        } else {
            panic!("Failed to initialize the device.");
        }
    }
}