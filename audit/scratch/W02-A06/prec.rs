// Verify operator precedence of `if fin { 0x80 } else { 0x00 } | opcode`
// exactly as written in control-protocol/src/frame.rs write_frame.
#[derive(Clone, Copy)]
enum Opcode { Text }
impl Opcode { const fn as_u8(self) -> u8 { match self { Opcode::Text => 1 } } }
fn main() {
    let fin = true;
    let opcode = Opcode::Text;
    let b: u8 = if fin { 0x80 } else { 0x00 } | opcode.as_u8();
    println!("fin+Text head byte = {:#04x}", b);
    assert_eq!(b, 0x81);
    let fin = false;
    let b: u8 = if fin { 0x80 } else { 0x00 } | opcode.as_u8();
    println!("!fin+Text head byte = {:#04x}", b);
    assert_eq!(b, 0x01);
}
