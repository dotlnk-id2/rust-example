use std::{marker::PhantomPinned, pin::Pin};


#[derive(Debug)]
pub struct Test {
    pub a: String,
    pub b: *const String,
    _marker: PhantomPinned,
}

impl Test {
    pub fn new(txt: &str) -> Self {
        // println!("txt.prt = {:?},v={}",txt.as_ptr(),txt);
        let me = Test {
            a: String::from(txt),
            b: std::ptr::null(),
            _marker: std::marker::PhantomPinned,
        };
        println!("Test a={:?}",me.a.as_ptr());
        me
    }

    pub fn init(self: Pin<&mut Self>) {
        let self_ptr: *const String = &self.a;
        let this = unsafe { self.get_unchecked_mut() };
        this.b = self_ptr;
    }

    pub fn a(self: Pin<&Self>) -> &str {
        &self.get_ref().a
    }

    pub fn b(self: Pin<&Self>) -> &String {
        assert!(!self.b.is_null(), "Test::b called without Test::init being called first");
        unsafe { &*(self.b) }
    }

    // fn init(&mut self) {
    //     let self_ref: *const String = &self.a;
    //     println!("init self_ref={:?},a={:?}",self_ref,&self.a);
    //     self.b = self_ref;
    // }

    // fn a(&self) -> &str {
    //     &self.a
    // }

    // fn b(&self) -> &String {
    //     assert!(!self.b.is_null(), "Test::b called without Test::init being called first");
    //     unsafe { &*(self.b) }
    // }
}