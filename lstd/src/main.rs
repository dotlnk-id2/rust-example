#[derive(Debug)]
enum Color {
    Rgb(i32, i32, i32),
    Hsv(i32, i32, i32),
}

#[derive(Debug)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(Color),
    Hello { id: i32 },
}

fn main() {
    let msg = Message::ChangeColor(Color::Hsv(0, 160, 255));

    match msg {
        Message::ChangeColor(Color::Rgb(r, g, b)) => {
            println!(
                "Change the color RGB to red {}, green {}, and blue {}",
                r, g, b
            )
        }
        Message::ChangeColor(Color::Hsv(h, s, v)) => {
            println!(
                "Change the color HSV to hue {}, saturation {}, and value {}",
                h, s, v
            )
        }
        _ => (),
    }

    let num = Some(10);

    match num {
        Some(x) if x < 5 => println!("less than five: {}", x),
        Some(x) if x > 5 && x < 10 => println!("greater than ten: {}", x),
        Some(x) => println!("between five and ten (inclusive): {}", x),
        None => (),
    }

    let x = Some(10);
    let mut y = 10;

    match x {
        Some(50) => println!("Got 50"),
        Some(n) if n == y => println!("Matched, n = {}", n),
        _ => {
            y = y + 1;
            println!("Default case, x = {:?}", x)
        }
    }

    println!("at the end: x = {:?}, y = {}", x, y);

    let x = 4;
    let y = true;

    match x {
        4 | 5 | 6 if y => println!("yes"),
        _ => println!("no"),
    }

    let xxx = Message::Hello { id: 1 };

    match xxx {
        Message::Hello {
            id: id_variable @ 3..=7,
        } => {
            println!("Found an id in range: {}", id_variable)
        }
        Message::Hello { id: 10..=12 } => {
            println!("Found an id in another range")
        }
        Message::Hello { id } => {
            println!("Found some other id: {}", id)
        }
        _ => println!("other {:?}", xxx),
    }
}
