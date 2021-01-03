`winput` is a high-level interface to *Windows*'s input system.

## Examples

The `Keylike` structure allows you to synthesize keystrokes on objects that can be used as keys.

```rust
use winput::{Keylike, Vk, Button};

// Synthesize keystrokes from a Virtual-Key Code
Vk::Shift.press().unwrap();    // press the shift key
Vk::A.send().unwrap();         // press then release the A key
Vk::Shift.release().unwrap();  // release the shift key

// Synthesize keystrokes from characters
'F'.send().unwrap();
'O'.send().unwrap();
'O'.send().unwrap();

// Synthesize keystrokes from mosue buttons
Button::Left.send().unwrap();

// You can synthesize keystrokes for the characters of a string
winput::send_str("Hello, world!");
```

The `Mouse` structure can be used to manipulate the mouse.

```rust
use winput::Mouse;

// Retreive the position of the mouse.
let (x, y) = Mouse::position().unwrap();

// Set the mouse position
//  ... in screen coordinates
Mouse::set_position(10, 10).unwrap();
//  ... in normalized absolute coordinates
Mouse::move_absolute(0.5, 0.5).unwrap();
//  ... relatively to the current cursor's position
Mouse::move_relative(100, 50).unwrap();

// Rotate the mouse wheel (vertically)
Mouse::scroll(1.5).unwrap();
//  ... or horizontally
Mouse::scrollh(-1.5).unwrap();
```

For more complicated input patterns, the `Input` structure can be used.

```rust
use winput::{Input, Vk, Action, MouseMotion};

// There is multiple ways to create an `Input`:
let inputs = [
    // ... from a character
    Input::from_char('a', Action::Press).unwrap(),
    // ... from a Virtual-Key Code
    Input::from_vk(Vk::A, Action::Release),
    // ... from a mouse motion
    Input::from_motion(MouseMotion::Relative { x: 100, y: 100 }),

    // additional constructors are available
];

let number_of_inputs_inserted = winput::send_inputs(&inputs);

assert_eq!(number_of_inputs_inserted, 3);
// hopefully!
```