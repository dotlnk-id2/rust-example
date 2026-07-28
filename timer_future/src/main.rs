fn main() {
    let v1 = "1111";
    let v2 = "2222";

    // println!("v1.prt = {:?}",v1.as_ptr());
    // println!("v2.prt = {:?}",v2.as_ptr());

    let mut test1 = crate::Test::new(v1);
    // println!("test1 >>> a: {}, b: {}, a = {:?}, b = {:?}", test1.a(), test1.b(), test1.a.as_ptr(), test1.b);
    test1.init();
    println!("test1 >>> a: {}, b: {}, a = {:?}, b = {:?}", test1.a(), test1.b(), test1.a.as_ptr(), test1.b);
    let mut test2 = crate::Test::new(v2);
    // println!("test2 >>> a: {}, b: {}, a = {:?}, b = {:?}", test2.a(), test2.b(), test2.a.as_ptr(), test2.b);
    test2.init();
    println!("test2 >>> a: {}, b: {}, a = {:?}, b = {:?}", test2.a(), test2.b(), test2.a.as_ptr(), test2.b);

    println!(" >>>>>>>>>>>>>>>> swap >>>>>>>>>>>>>>>> ");

    std::mem::swap(&mut test1, &mut test2);

    println!("test1 >>> a: {}, b: {}, a = {:?}, b = {:?}", test1.a(), test1.b(), test1.a.as_ptr(), test1.b);
    println!("test2 >>> a: {}, b: {}, a = {:?}, b = {:?}", test2.a(), test2.b(), test2.a.as_ptr(), test2.b);

    // println!("test1 >>> a: {}", test1.a());
    // println!("test2 >>> a: {}", test2.a());

}


#[derive(Debug)]
pub(crate) struct Test {
    a: String,
    b: *const String,
}

impl Test {
    fn new(txt: &str) -> Self {
        println!("txt.prt = {:?},v={}",txt.as_ptr(),txt);
        Test {
            a: String::from(txt),
            b: std::ptr::null(),
        }
    }

    fn init(&mut self) {
        let self_ref: *const String = &self.a;
        self.b = self_ref;
    }

    fn a(&self) -> &str {
        &self.a
    }

    fn b(&self) -> &String {
        assert!(!self.b.is_null(), "Test::b called without Test::init being called first");
        unsafe { &*(self.b) }
    }
}