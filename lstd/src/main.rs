// #[derive(Debug)]
// enum Color {
//     Rgb(i32, i32, i32),
//     Hsv(i32, i32, i32),
// }

// #[derive(Debug)]
// enum Message {
//     Quit,
//     Move { x: i32, y: i32 },
//     Write(String),
//     ChangeColor(Color),
//     Hello { id: i32 },
// }

// fn main() {
//     use std::cell::Cell;

//     let c = Cell::new("asdf");
//     let one = c.get();
//     c.set("qwer");
//     let two = c.get();
//     println!("{},{}", one, two);

//     let x = Cell::new(1);
//     let y = &x;
//     let z = &x;
//     x.set(2);
//     y.set(3);
//     z.set(4);
//     println!("{}", x.get());

//     let msg = Message::ChangeColor(Color::Hsv(0, 160, 255));

//     match msg {
//         Message::ChangeColor(Color::Rgb(r, g, b)) => {
//             println!(
//                 "Change the color RGB to red {}, green {}, and blue {}",
//                 r, g, b
//             )
//         }
//         Message::ChangeColor(Color::Hsv(h, s, v)) => {
//             println!(
//                 "Change the color HSV to hue {}, saturation {}, and value {}",
//                 h, s, v
//             )
//         }
//         _ => (),
//     }

//     let num = Some(10);

//     match num {
//         Some(x) if x < 5 => println!("less than five: {}", x),
//         Some(x) if x > 5 && x < 10 => println!("greater than ten: {}", x),
//         Some(x) => println!("between five and ten (inclusive): {}", x),
//         None => (),
//     }

//     let x = Some(10);
//     let mut y = 10;

//     match x {
//         Some(50) => println!("Got 50"),
//         Some(n) if n == y => println!("Matched, n = {}", n),
//         _ => {
//             y = y + 1;
//             println!("Default case, x = {:?}", x)
//         }
//     }

//     println!("at the end: x = {:?}, y = {}", x, y);

//     let x = 4;
//     let y = true;

//     match x {
//         4 | 5 | 6 if y => println!("yes"),
//         _ => println!("no"),
//     }

//     let xxx = Message::Hello { id: 1 };

//     match xxx {
//         Message::Hello {
//             id: id_variable @ 3..=7,
//         } => {
//             println!("Found an id in range: {}", id_variable)
//         }
//         Message::Hello { id: 10..=12 } => {
//             println!("Found an id in another range")
//         }
//         Message::Hello { id } => {
//             println!("Found some other id: {}", id)
//         }
//         _ => println!("other {:?}", xxx),
//     }

//     let mut v: Vec<i32> = vec![1, 2, 3];
//     println!("after {:?}", v);
//     for i in &mut v {
//         *i += 10
//     }
//     println!("befort {:?}", v);

//     let mut v = Vec::with_capacity(10);
//     v.extend([1, 2, 3]); // 附加数据到 v
//     println!("Vector 长度是: {}, 容量是: {}", v.len(), v.capacity());

//     v.reserve(100); // 调整 v 的容量，至少要有 100 的容量
//     println!(
//         "Vector（reserve） 长度是: {}, 容量是: {}",
//         v.len(),
//         v.capacity()
//     );

//     v.shrink_to_fit(); // 释放剩余的容量，一般情况下，不会主动去释放容量
//     println!(
//         "Vector（shrink_to_fit） 长度是: {}, 容量是: {}",
//         v.len(),
//         v.capacity()
//     );

//     let lt = "test";

//     println!(
//         "lifetime p = {:?} , r = {:?}",
//         &lt,
//         |fv: &'static str| -> &'static str { fv }(&lt)
//     );

//     let mut s = String::new();

//     let mut update_string = |str| s.push_str(str);
//     update_string("hello");

//     println!("{:?}", s);

//     let arr = [1, 2, 3];
//     let mut arr_iter = arr.into_iter();

//     assert_eq!(arr_iter.next(), Some(1));
//     assert_eq!(arr_iter.next(), Some(2));
//     assert_eq!(arr_iter.next(), Some(3));
//     assert_eq!(arr_iter.next(), None);
//     assert_eq!(arr_iter.next(), None);
//     assert_eq!(arr_iter.next(), None);


// }


use std::{slice::from_raw_parts, str::from_utf8_unchecked};

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, FmtSubscriber, filter::Directive, fmt, layer::SubscriberExt, util::SubscriberInitExt};

// 获取字符串的内存地址和长度
fn get_memory_location() -> (usize, usize) {
  let string = "Hello World!";
  let pointer = string.as_ptr() as usize;
  let length = string.len();
  (pointer, length)
}

// 在指定的内存地址读取字符串
fn get_str_at_location(pointer: usize, length: usize) -> &'static str {
  unsafe { from_utf8_unchecked(from_raw_parts(pointer as *const u8, length)) }
}

fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    // 1. 建立按天切割的 File Appender (日誌將儲存於 ./logs/app.log.YYYY-MM-DD)
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY) // 支持 HOURLY, DAILY, NEVER
        .filename_prefix("app")
        .filename_suffix("log")
        .build("logs")
        .expect("無法建立日誌目錄");

    // 2. 包裹為非阻塞 Appender (返回 WorkerGuard 確保程式退出前 Flush 緩衝區)
    let (non_blocking_appender, guard) = tracing_appender::non_blocking(file_appender);

    // 3. 定義過濾規則
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 4. 定義檔案輸出 Layer (使用 JSON 格式，便於日志收集系统解析)
    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking_appender);

    // 5. 定義控制台輸出 Layer (使用易讀的彩色格式)
    let stdout_layer = fmt::layer()
        .pretty()
        .with_writer(std::io::stdout);

    // 6. 組合並註冊全域 Subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    guard // 必須將 Guard 返回並持有一直到 main 結束
}

fn main() {

  let _g = init_tracing();

    tracing::info!("服務已啟動");

    tracing::info!("OS: {}", std::env::consts::OS);
    tracing::info!("Arch: {}", std::env::consts::ARCH);
    tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Concurrent: {}", 4);
    tracing::info!("Connect timeout: {:?}s",11);

    tracing::error_span!()


  let (pointer, length) = get_memory_location();
  let message = get_str_at_location(pointer, length);
  println!(
    "The {} bytes at 0x{:X} stored: {}",
    length, pointer, message
  );
  // 如果大家想知道为何处理裸指针需要 `unsafe`，可以试着反注释以下代码
  let message = get_str_at_location(1000, 10);
  println!("{:?}",message)
}


trait Test {
    #[warn(async_fn_in_trait)]
    async fn test();
}