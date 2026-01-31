// Model of the data read in this app

use heapless::String;

pub struct Model {
    pub temperature: f32,
    pub ip_address: String<16>,
}
