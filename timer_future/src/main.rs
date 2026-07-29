use std::{marker::PhantomPinned, pin::Pin};

fn main() {
    use timer_future::Test;

    let v1 = "1111";
    let v2 = "2222";

    println!("v1.prt = {:?}",v1.as_ptr());
    println!("v2.prt = {:?}",v2.as_ptr());

    let mut test1 = timer_future::Test::new(v1);
    let mut test1 = unsafe { Pin::new_unchecked(&mut test1) };
    timer_future::Test::init(test1.as_mut());
    // test1.init();
    println!("test1 >>> a: {}, b: {}, a = {:?}, b = {:?}", Test::a(test1.as_ref()), Test::b(test1.as_ref()), test1.a.as_ptr(), test1.b);
    let mut test2 = timer_future::Test::new(v2);
    let mut test2 = unsafe { Pin::new_unchecked(&mut test2) };
    Test::init(test2.as_mut());
    // test2.init();
    println!("test2 >>> a: {}, b: {}, a = {:?}, b = {:?}", Test::a(test2.as_ref()), Test::b(test2.as_ref()), test2.a.as_ptr(), test2.b);


    println!(" >>>>>>>>>>>>>>>> swap >>>>>>>>>>>>>>>> ");

    // std::mem::swap(&mut test1, &mut test2);
    // std::mem::swap(test1.get_mut(), &mut test2.get_mut());

    println!("test1 >>> a: {}, b: {}, a = {:?}, b = {:?}", Test::a(test1.as_ref()), Test::b(test1.as_ref()), test1.a.as_ptr(), test1.b);
    println!("test2 >>> a: {}, b: {}, a = {:?}, b = {:?}", Test::a(test2.as_ref()), Test::b(test2.as_ref()), test2.a.as_ptr(), test2.b);


}

