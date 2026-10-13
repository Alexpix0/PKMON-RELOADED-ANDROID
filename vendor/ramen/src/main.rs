use ramen::{connection::Connection, event::Event, window::Cursor}; // There's no actual error here it's an RA bug sorry in advance

#[cfg(feature = "input")]
use ramen::input::Key;

pub fn main() {
    let c = match Connection::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error setting up connection: {:?}", e);
            return;
        },
    };

    f(c);

    // let cs = [c.clone(), c.clone(), c.clone(), c.clone(), c.clone()];
    // let t = cs.map(|c| std::thread::spawn(move || f(c)));
    // for x in t {
    //     if x.join().is_err() {
    //         eprintln!("Exiting main thread because join() failed");
    //         return;
    //     }
    // }
}

pub fn f(connection: Connection) {
    let mut borderless = false;
    let mut resizable = false;
    let mut window = match connection
        .into_builder()
        .controls(Some(ramen::window::Controls::new()))
        .borderless(borderless)
        .class_name("OpenGMK")
        .resizable(resizable)
        .title("simple window, חלון הומו טיפש,彼の死を心から願っています🙏")
        .maximised(false)
        .position(None)
        .size((800, 608))
        .visible(true)
        .build()
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error building window: {:?}", e);
            return;
        },
    };
    'program: loop {
        window.poll_events();
        for event in window.events() {
            match event {
                Event::CloseRequest => {
                    println!("Closed.");
                    break 'program;
                },
                Event::Focus(b) => {
                    println!("Window focus state: {}", b);
                },
                Event::Maximise(v) => {
                    println!("!! Maximise: {}", v);
                },
                Event::Minimise(v) => {
                    println!("!! Minimise: {}", v);
                },
                Event::Move(t) => {
                    println!("Window move: {:?}", t);
                },
                Event::Resize(t) => {
                    println!("Window resize: {:?}", t);
                },
                Event::Visible(t) => {
                    println!("Window{} visible", if *t { "" } else { " not" });
                },
                #[cfg(feature = "input")]
                Event::KeyboardDown(k) => {
                    println!("Key down: {:?}", k);
                    match k {
                        Key::T => window.set_title("This is a different title"),
                        Key::M => window.set_maximised(true),
                        Key::N => window.set_maximised(false),
                        Key::P => window.set_position((10, 10)),
                        Key::S => window.set_size((800, 608)),
                        Key::R => {
                            resizable = !resizable;
                            window.set_resizable(resizable);
                        },
                        Key::B => {
                            borderless = !borderless;
                            window.set_borderless(borderless);
                        },
                        Key::C => {
                            window.set_cursor(Cursor::Cross);
                        },
                        _ => (),
                    }
                },
                #[cfg(feature = "input")]
                Event::KeyboardRepeat(k) => {
                    println!("Key repeat: {:?}", k);
                },
                #[cfg(feature = "input")]
                Event::KeyboardUp(k) => {
                    println!("Key up: {:?}", k);
                },
                #[cfg(feature = "input")]
                Event::MouseDown(k) => {
                    println!("Mouse down: {:?}", k);
                },
                #[cfg(feature = "input")]
                Event::MouseUp(k) => {
                    println!("Mouse up: {:?}", k);
                },
                #[cfg(feature = "input")]
                Event::MouseMove(t) => {
                    println!("Mouse move: {:?}", t);
                },
                #[cfg(feature = "input")]
                Event::ScrollUp => {
                    println!("Mouse scrolled up");
                },
                #[cfg(feature = "input")]
                Event::ScrollDown => {
                    println!("Mouse scrolled down");
                },
                #[cfg(feature = "input")]
                Event::Input(code) => {
                    println!("Input: {}", code);
                },
                #[cfg(feature = "input")]
                Event::MouseEnter => println!("!! Mouse enter"),
                #[cfg(feature = "input")]
                Event::MouseLeave => println!("!! Mouse leave"),
                _ => (),
            }
        }
    }
}
