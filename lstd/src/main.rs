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
    use std::cell::Cell;

    let c = Cell::new("asdf");
    let one = c.get();
    c.set("qwer");
    let two = c.get();
    println!("{},{}", one, two);

    let x = Cell::new(1);
let y = &x;
let z = &x;
x.set(2);
y.set(3);
z.set(4);
println!("{}", x.get());

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

    let mut v: Vec<i32> = vec![1, 2, 3];
    println!("after {:?}", v);
    for i in &mut v {
        *i += 10
    }
    println!("befort {:?}", v);

    let mut v = Vec::with_capacity(10);
    v.extend([1, 2, 3]); // 附加数据到 v
    println!("Vector 长度是: {}, 容量是: {}", v.len(), v.capacity());

    v.reserve(100); // 调整 v 的容量，至少要有 100 的容量
    println!(
        "Vector（reserve） 长度是: {}, 容量是: {}",
        v.len(),
        v.capacity()
    );

    v.shrink_to_fit(); // 释放剩余的容量，一般情况下，不会主动去释放容量
    println!(
        "Vector（shrink_to_fit） 长度是: {}, 容量是: {}",
        v.len(),
        v.capacity()
    );

    let lt = "test";

    println!(
        "lifetime p = {:?} , r = {:?}",
        &lt,
        |fv: &'static str| -> &'static str { fv }(&lt)
    );

    let mut s = String::new();

    let mut update_string = |str| s.push_str(str);
    update_string("hello");

    println!("{:?}", s);

    let arr = [1, 2, 3];
    let mut arr_iter = arr.into_iter();

    assert_eq!(arr_iter.next(), Some(1));
    assert_eq!(arr_iter.next(), Some(2));
    assert_eq!(arr_iter.next(), Some(3));
    assert_eq!(arr_iter.next(), None);
    assert_eq!(arr_iter.next(), None);
    assert_eq!(arr_iter.next(), None);
}

// fn main() {
//     let lt = "test";

//     println!(
//         "lifetime p = {:?} , r = {:?}",
//         &lt,
//         |fv: &'static str| -> &'static str { fv }(&lt)
//     );
// }
