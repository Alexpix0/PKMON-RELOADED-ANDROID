// Temporary diagnostic tool: dumps object event code that mentions a given string.
use gm8exe::reader;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file = std::fs::read(&args[1]).expect("read exe");
    let assets = reader::from_exe(&mut file, None::<fn(&str)>, false, false).expect("load exe");
    let needle = args.get(2).cloned().unwrap_or_else(|| "mouse".to_string());
    for (id, object) in assets.objects.iter().enumerate() {
        let Some(object) = object else { continue };
        let name = String::from_utf8_lossy(object.name.0.as_ref()).to_string();
        for (list, subs) in object.events.iter().enumerate() {
            for (sub, actions) in subs {
                for action in actions {
                    let mut code = String::from_utf8_lossy(action.fn_code.0.as_ref()).to_string();
                    for parameter in &action.param_strings {
                        let text = String::from_utf8_lossy(parameter.0.as_ref());
                        if !text.trim().is_empty() {
                            code.push_str(&text);
                        }
                    }
                    if code.contains(&needle) {
                        println!("=== obj#{id} '{name}' event#{list}.{sub} ===\n{code}\n");
                    }
                }
            }
        }
    }
}
