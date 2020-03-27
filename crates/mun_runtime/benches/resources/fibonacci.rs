pub fn fibonacci(n: i64) -> i64 {
    let mut a = 0;
    let mut b = 1;
    let mut i = 1;
    loop {
        if i > n {
            return a;
        }
        let sum = a + b;
        a = b;
        b = sum;
        i += 1;
    }
}

pub fn fibonacci_main(n:i64) -> i64 {
    fibonacci(n)
}