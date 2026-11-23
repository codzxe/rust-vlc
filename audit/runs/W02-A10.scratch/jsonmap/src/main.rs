fn main() {
    let a: Vec<&str> = "abc\n".lines().collect();
    let b: Vec<&str> = "abc".lines().collect();
    println!("with-newline: {:?}  without: {:?}  equal: {}", a, b, a == b);
    let c: Vec<&str> = "x\r\ny".lines().collect();
    println!("crlf: {:?}", c);
}
