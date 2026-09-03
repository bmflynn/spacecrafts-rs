use std::collections::HashMap;

#[cfg(feature = "serde")]
use std::{fs::File, path::Path};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Apid, Error, Result, Vcid};

// Standard CCSDS ASM length
const ASM_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum FrameType {
    CcsdsAos,
    CcsdsTm,
    None,
}

impl FrameType {
    fn default() -> Self {
        Self::CcsdsAos
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RS {
    pub interleave: usize,
    #[cfg_attr(feature = "serde", serde(default))]
    pub virtual_fill_length: usize,
    pub num_correctable: u32,
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PNConfig {}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FramingConfig {
    pub length: usize,
    pub asm: Option<Vec<u8>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub fhec_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ocf_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub fec_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub izone_length: usize,
    pub pseudo_noise: Option<PNConfig>,
    #[cfg_attr(
        feature = "serde",
        serde(rename = "type", default = "FrameType::default")
    )]
    pub frame_type: FrameType,
    pub reed_solomon: Option<RS>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChannelFramingConfig {
    #[cfg_attr(feature = "serde", serde(default))]
    pub fhec_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ocf_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub fec_present: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub izone_length: usize,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FrameChannel {
    pub vcid: Vcid,
    pub framing: Option<ChannelFramingConfig>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PacketChannel {
    pub apid: Apid,
    pub vcid: Vcid,
    /// Name of global timecode configuration.
    pub timecode: Option<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase", tag = "format"))]
pub enum TimecodeConfig {
    CDS {
        epoch: String,
        #[cfg_attr(feature = "serde", serde(rename = "dayLength", default))]
        day_length: usize,
        #[cfg_attr(feature = "serde", serde(rename = "submillisLength", default))]
        submillis_length: usize,
        #[cfg_attr(feature = "serde", serde(rename = "selfIdentifying", default))]
        self_identifying: bool,
    },
    CUC {
        epoch: String,
        #[cfg_attr(feature = "serde", serde(rename = "basicLength", default))]
        basic_length: usize,
        #[cfg_attr(feature = "serde", serde(rename = "fineLength", default))]
        fine_length: usize,
        #[cfg_attr(feature = "serde", serde(rename = "fineNanos", default))]
        fine_nanos: u32,
        #[cfg_attr(feature = "serde", serde(rename = "selfIdentifying", default))]
        self_identifying: bool,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Spacecraft {
    pub scid: u16,
    /// The canonical identifier for the satellite.
    pub norad_catalog_id: u32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub aliaes: Vec<String>,
    pub framing: FramingConfig,
    #[cfg_attr(feature = "serde", serde(default))]
    pub vcids: Vec<FrameChannel>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub apids: Vec<PacketChannel>,
    #[cfg_attr(feature = "serde", serde(default))]
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
                    if !self_identifying {
                        if *day_length == 0 {
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
                    if !self_identifying {
                        if *basic_length == 0 {
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
