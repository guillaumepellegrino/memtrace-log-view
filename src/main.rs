use clap::{Parser};
use std::fs::File;
use std::io::{self, BufRead};
use chrono::{NaiveDateTime};
use eyre::WrapErr;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// View a memtrace log file with dataviewer
struct Args {
    #[arg()]
    /// Path to memtrace.log file
    file: std::path::PathBuf,
}

#[derive(Debug, PartialEq, Default, Copy, Clone, Deserialize, Serialize)]
pub enum Type {
    #[default]
    XY,
    Line,
}

#[derive(Debug, PartialEq, Default, Clone, Deserialize, Serialize)]
pub struct DataView {
    pub r#type: Type,
    pub title: Option<String>,
    pub x_title: Option<String>,
    pub y_title: Option<String>,
    pub x_unit: Option<String>,
    pub y_unit: Option<String>,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, Default, Clone, Deserialize, Serialize)]
pub struct Chart {
    pub title: Option<String>,
    pub description: Option<String>,
}

/// The root definition of a DataView File
#[derive(Debug, PartialEq, Default, Clone, Deserialize, Serialize)]
pub struct DataViewer {
    #[serde(default)]
    pub dataview: DataView,

    #[serde(default)]
    pub chart: HashMap<String, Chart>,

    #[serde(default)]
    pub data: HashMap<String, Vec<f64>>,
}

impl DataViewer {
    fn new() -> Self {
        let mut me = Self::default();
        me.dataview.title = Some("History of memory usage with Memtrace".into());
        me.dataview.x_title = Some("Elapsed Time".into());
        me.dataview.x_unit = Some("Hour".into());
        me.dataview.y_title = Some("Memory in use".into());
        me.dataview.y_unit = Some("KBytes".into());

        let mut chart = Chart::default();
        chart.title = Some("Memory in use".into());

        me.chart.insert("inuse".into(), chart);
        me
    }

    pub fn write(&self, path: &std::path::Path) -> eyre::Result<()> {
        let toml = toml::to_string(&self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    fn add(&mut self, elapsed_sec: i64, inuse: &str) -> eyre::Result<()> {
        let regex = Regex::new(r"(\d+) bytes").unwrap();
        let captures = regex.captures(inuse).unwrap();
        let bytes = captures.get(1).unwrap().as_str();
        let bytes = bytes.parse::<f64>()?;
        let kbytes = bytes / 1000.0;
        let elapsed_hour = (elapsed_sec as f64)/(60.0*60.0);

        let values = self.data.entry("inuse".into())
            .or_insert_with(|| vec![]);
        values.push(elapsed_hour);
        values.push(kbytes);

        Ok(())
    }
}

fn main() -> eyre::Result<()> {
    let args = Args::parse();
    let file = File::open(&args.file)?;
    let fileout = args.file.with_extension("log.toml");
    let lines = io::BufReader::new(file).lines();
    let mut dataviewer = DataViewer::new();
    let mut timestamp = 0;
    let mut min_ts = 0;

    for line in lines {
        let line = line?;

        if let Some(date) = line.strip_prefix("HEAP SUMMARY ") {
            let format = "%a %b %d %H:%M:%S %Y";
            let date = NaiveDateTime::parse_from_str(date, format)
                .wrap_err(format!("Error parsing '{}'", date))?;
            timestamp = date.and_utc().timestamp();
            if min_ts == 0 {
                min_ts = timestamp;
            }
        }
        if let Some(inuse) = line.strip_prefix("    in use: ") {
            let elapsed = timestamp - min_ts;
            dataviewer.add(elapsed, inuse)?;
        }
    }

    dataviewer.write(&fileout)?;
    println!("Write dataviewer file to {:?}", fileout);

    println!("Starting: dataviewer {:?}", fileout);
    std::process::Command::new("dataviewer")
        .arg(fileout)
        .output()?;

    Ok(())
}
