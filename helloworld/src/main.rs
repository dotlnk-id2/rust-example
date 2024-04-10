fn main() {
    println!("Hello, world!");
    printWorld();
}


fn printWorld(){
    let en = "hello,world!";
    let zh = "你好世界！";
    let jp = "ohayou!";

    let array:[&str;3] =[en,zh,jp];

    for p in array.iter(){
        println!("printWorld={}",p);
    }
}