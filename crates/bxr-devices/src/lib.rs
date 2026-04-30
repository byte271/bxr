#![forbid(unsafe_code)]

use std::collections::VecDeque;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceId(pub &'static str);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceSnapshot {
    pub device_id: DeviceId,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceError {
    WrongDevice {
        expected: DeviceId,
        actual: DeviceId,
    },
}

pub trait Device {
    fn id(&self) -> DeviceId;
    fn reset(&mut self);
    fn snapshot(&self) -> DeviceSnapshot;
    fn restore(&mut self, snapshot: &DeviceSnapshot) -> Result<(), DeviceError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SerialDevice {
    output: Vec<u8>,
    input: VecDeque<u8>,
}

impl SerialDevice {
    pub const DEVICE_ID: DeviceId = DeviceId("serial0");
    pub const DEBUG_CONSOLE_PORT: u16 = 0x00e9;
    pub const COM1_DATA_PORT: u16 = 0x03f8;

    pub fn write_byte(&mut self, byte: u8) {
        self.output.push(byte);
    }

    pub fn queue_input(&mut self, byte: u8) {
        self.input.push_back(byte);
    }

    pub fn read_input(&mut self) -> Option<u8> {
        self.input.pop_front()
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }

    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output)
    }
}

impl Device for SerialDevice {
    fn id(&self) -> DeviceId {
        Self::DEVICE_ID
    }

    fn reset(&mut self) {
        self.output.clear();
        self.input.clear();
    }

    fn snapshot(&self) -> DeviceSnapshot {
        let mut payload = self.output.clone();
        payload.push(0);
        payload.extend(self.input.iter());
        DeviceSnapshot {
            device_id: Self::DEVICE_ID,
            payload,
        }
    }

    fn restore(&mut self, snapshot: &DeviceSnapshot) -> Result<(), DeviceError> {
        if snapshot.device_id != Self::DEVICE_ID {
            return Err(DeviceError::WrongDevice {
                expected: Self::DEVICE_ID,
                actual: snapshot.device_id.clone(),
            });
        }

        let split = snapshot
            .payload
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(snapshot.payload.len());
        self.output = snapshot.payload[..split].to_vec();
        self.input = snapshot
            .payload
            .get(split + 1..)
            .unwrap_or_default()
            .iter()
            .copied()
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_snapshots_output_and_input() {
        let mut serial = SerialDevice::default();
        serial.write_byte(b'O');
        serial.queue_input(b'I');

        let snapshot = serial.snapshot();
        serial.reset();
        serial.restore(&snapshot).unwrap();

        assert_eq!(serial.output(), b"O");
        assert_eq!(serial.read_input(), Some(b'I'));
    }
}
