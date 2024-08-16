use std::mem;
use std::net::{TcpStream, Shutdown};
use std::io::{Result, Write};
use crate::util::Color;

pub struct Params {
    pub max_b: u32,
    pub num_l: u32,
    pub mul_r: f32,
    pub mul_g: f32,
    pub mul_b: f32,
    pub lerp: f32,
    pub rate: u32,
}

impl Params {
    fn to_be_bytes(&self) -> Vec<u8> {
        vec![
            self.max_b.to_be_bytes(),
            self.num_l.to_be_bytes(),
            self.mul_r.to_be_bytes(),
            self.mul_g.to_be_bytes(),
            self.mul_b.to_be_bytes(),
            self.lerp.to_be_bytes(),
            self.rate.to_be_bytes(),
        ].concat()
    }
}

pub struct Connection {
    stream: TcpStream,
    pub colors: Vec<Color>
}

pub fn connect(addr: &str, params: &Params) -> Result<Connection> {
    let mut stream = TcpStream::connect(addr)?;
    stream.write(&params.to_be_bytes())?;

    Ok(Connection {
        stream,
        colors: vec![Color::default(); params.num_l as usize],
    })
}

pub fn update(conn: &mut Connection) -> Result<()> {
    let mut buf = Vec::with_capacity(mem::size_of::<u8>() * 3 * conn.colors.len());
    for color in &conn.colors {
        buf.extend_from_slice(&color.r.to_be_bytes());
        buf.extend_from_slice(&color.g.to_be_bytes());
        buf.extend_from_slice(&color.b.to_be_bytes());
    }
    conn.stream.write(&buf)?;

    Ok(())
}

pub fn close(conn: &Connection) -> Result<()> {
    conn.stream.shutdown(Shutdown::Both)
}
