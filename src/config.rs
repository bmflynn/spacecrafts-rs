use std::collections::HashMap;

#[cfg(feature = "serde")]
use std::{fs::File, path::Path};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Apid, Error, Result, Vcid};

// Standard CCSDS ASM length
const ASM_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FrameType {
    CcsdsAos,
    CcsdsTm,
    None,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RS {
    pub interleave: usize,
    pub virtual_fill_length: Option<usize>,
    pub num_correctable: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct PNConfig {}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FramingConfig {
    pub length: usize,
    pub fhec_present: Option<bool>,
    pub ocf_present: Option<bool>,
    pub fec_present: Option<bool>,
    pub izone_length: Option<usize>,
    pub pseudo_noise: Option<PNConfig>,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub frame_type: FrameType,
    pub reed_solomon: Option<RS>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChannelFramingConfig {
    pub fhec_present: Option<bool>,
    pub ocf_present: Option<bool>,
    pub fec_present: Option<bool>,
    pub izone_length: Option<usize>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FrameChannel {
    pub vcid: Vcid,
    pub framing: Option<ChannelFramingConfig>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PacketChannel {
    pub apid: Apid,
    pub vcid: Vcid,
    pub timecode: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase", tag = "format"))]
pub enum TimecodeConfig {
    CDS {
        epoch: String,
        #[cfg_attr(feature = "serde", serde(rename = "dayLength"))]
        day_length: Option<usize>,
        #[cfg_attr(feature = "serde", serde(rename = "submillisLength"))]
        submillis_length: Option<usize>,
        #[cfg_attr(feature = "serde", serde(rename = "selfIdentifying"))]
        self_identifying: Option<bool>,
    },
    CUC {
        epoch: String,
        #[cfg_attr(feature = "serde", serde(rename = "basicLength"))]
        basic_length: Option<usize>,
        #[cfg_attr(feature = "serde", serde(rename = "fineLength"))]
        fine_length: Option<usize>,
        #[cfg_attr(feature = "serde", serde(rename = "fineNanos"))]
        fine_nanos: Option<u32>,
        #[cfg_attr(feature = "serde", serde(rename = "selfIdentifying"))]
        self_identifying: Option<bool>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct Spacecraft {
    pub scid: u16,
    pub name: String,
    pub norad_catalog_id: u32,
    pub aliaes: Option<Vec<String>>,
    pub framing: FramingConfig,
    pub vcids: Vec<FrameChannel>,
    pub apids: Vec<PacketChannel>,
    pub timecodes: HashMap<String, TimecodeConfig>,
}

impl Spacecraft {
    #[cfg(feature = "serde")]
    pub fn with_file<P: AsRef<Path>>(path: P) -> Result<Spacecraft> {
        config_with_file(path)
    }

    fn validate_timecodes(&self) -> Result<()> {
        for (key, tc) in self.timecodes.iter() {
            match tc {
                TimecodeConfig::CDS {
                    day_length,
                    self_identifying,
                    ..
                } => {
                    if !self_identifying.unwrap_or(false) {
                        if day_length.is_none() {
                            return Err(Error::Invalid(format!(
                                "timecode config {key} requires at least day length and epoch"
                            )));
                        }
                    }
                }
                TimecodeConfig::CUC {
                    basic_length,
                    self_identifying,
                    ..
                } => {
                    if !self_identifying.unwrap_or(false) {
                        if basic_length.is_none() {
                            return Err(Error::Invalid(format!(
                                "timecode config {key} requires at least basic length and epoch"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_apids(&self) -> Result<()> {
        for (i, apid) in self.apids.iter().enumerate() {
            let Some(tcname) = &apid.timecode else {
                continue;
            };
            if self.timecodes.get(tcname).is_none() {
                return Err(Error::Invalid(format!(
                    "apid index {i} references unconfigured timecode {tcname}"
                )));
            };
        }
        Ok(())
    }

    fn validate_vcids(&self) -> Result<()> {
        for ch in self.vcids.iter() {
            if ch.framing.is_some() {
                return Err(Error::Invalid(format!(
                    "per-channel framing config (vcid={}) not currenty supported",
                    ch.vcid
                )));
            };
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_timecodes()?;
        self.validate_apids()?;
        self.validate_vcids()?;
        Ok(())
    }

    /// The total length of a CADU for this configuration. This will include the length
    /// of the ASM, the frame, and RS parity bytes if RS is used.
    pub fn cadu_length(&self) -> usize {
        let mut cadu_len = ASM_LEN + self.framing.length;
        if let Some(rs) = &self.framing.reed_solomon {
            // byte length of RS parity
            cadu_len += rs.interleave * (rs.num_correctable as usize * 2);
        }
        cadu_len
    }
}

#[cfg(feature = "serde")]
fn config_with_file<P: AsRef<Path>>(path: P) -> Result<Spacecraft> {
    let reader = File::open(&path)?;
    let cfg: Spacecraft = serde_json::from_reader(reader)?;

    Ok(cfg)
}
