use std::error::Error;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use visa_rs::{flags::AccessMode, DefaultRM, Instrument, TIMEOUT_IMMEDIATE};

fn query(mut instr: &Instrument, query: &[u8]) -> Result<String, Box<dyn Error>> {
    instr.write_all(query)?;
    let mut buf_reader = BufReader::new(instr);
    let mut buf = String::new();
    let _ = buf_reader.read_to_string(&mut buf);
    Ok(buf)
}

fn idn(visa_address: &str) -> Result<(), Box<dyn Error>> {
    let rm = DefaultRM::new()?;
    let rsc = CString::new(visa_address)?.into();
    let instr = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
    let response = query(&instr, b"*IDN?")?;
    println!("{:?}", response.trim());
    Ok(())
}

fn get_screenshot(visa_address: &str, filename: &str) -> Result<(), Box<dyn Error>> {
    let rm = DefaultRM::new()?;
    let rsc = CString::new(visa_address)?.into();
    let mut instr = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
    instr.write_all(b":HARDcopy:INKSaver OFF")?;
    instr.write_all(b":DISP:DATA? PNG, COLor")?;
    let mut buf_reader = BufReader::new(instr);
    let mut buf = Vec::new();
    while buf.is_empty() {
        let _ = buf_reader.read_to_end(&mut buf);
    }
    let mut file = File::create(filename)?;
    file.write_all(&buf[10..buf.len() - 1])?;
    file.flush()?;
    Ok(())
}

fn get_csv(visa_address: &str, filename: &str, ch: u64) -> Result<(), Box<dyn Error>> {
    let rm = DefaultRM::new()?;
    let rsc = CString::new(visa_address)?.into();
    let mut instr = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
    instr.write_all(b":WAVeform:POINts:MODE RAW")?;
    instr.write_all(b":WAVeform:POINts 10240")?;
    instr.write_all(format!(":WAVeform:SOURce CHANnel{}", ch).as_bytes())?;
    instr.write_all(b":WAVeform:FORMat BYTE")?;

    let preamble_string = query(&instr, b":WAVeform:PREamble?")?;
    let preamble: Vec<f64> = preamble_string
        .as_str()
        .trim()
        .split(',')
        .map(|x| x.trim().parse().unwrap_or(0.0))
        .collect();

    let x_increment = preamble[4];
    let x_origin = preamble[5];
    let y_increment = preamble[7];
    let y_origin = preamble[8];
    let y_ref = preamble[9];

    instr.write_all(b":WAVeform:DATA?")?;
    let mut buf_reader = BufReader::new(instr);
    let mut buf = Vec::new();
    while buf.is_empty() {
        let _ = buf_reader.read_to_end(&mut buf);
    }

    let mut wtr = csv::Writer::from_path(filename)?;
    wtr.write_record(["time", "data"])?;
    for (i, value) in buf[10..buf.len() - 1].iter().enumerate() {
        let time = x_origin + i as f64 * x_increment;
        let data = (*value as f64 - y_ref) * y_increment + y_origin;
        wtr.write_record([time.to_string(), data.to_string()])?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let visa_address = "TCPIP0::192.168.3.242::hislip0::INSTR";
    idn(visa_address)?;
    get_screenshot(visa_address, "waveform.png")?;
    get_csv(visa_address, "waveform.csv", 1)?;
    Ok(())
}
