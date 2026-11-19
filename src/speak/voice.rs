//! Kokoro speaker catalog.
//!
//! `voices.bin` stores the speaker embeddings but no names, and sherpa-onnx
//! only exposes a numeric `sid`. These tables are the published name-to-id
//! maps for the two multilingual bundles, so a caller can ask for
//! `"af_maple"` instead of remembering that it is speaker 0. The catalog is
//! selected by the speaker count the loaded model reports.

/// One Kokoro speaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voice {
    /// Speaker id passed to sherpa-onnx as `sid`.
    pub sid: i32,
    /// Published voice name (`af_`, `am_`, `bf_`, ... prefixes are the
    /// language and gender, as in Kokoro's `VOICES.md`).
    pub name: &'static str,
}

/// `kokoro-multi-lang-v1_0`: 54 speakers, Chinese + English.
pub const V1_0: &[Voice] = &[
    Voice {
        sid: 0,
        name: "af_alloy",
    },
    Voice {
        sid: 1,
        name: "af_aoede",
    },
    Voice {
        sid: 2,
        name: "af_bella",
    },
    Voice {
        sid: 3,
        name: "af_heart",
    },
    Voice {
        sid: 4,
        name: "af_jessica",
    },
    Voice {
        sid: 5,
        name: "af_kore",
    },
    Voice {
        sid: 6,
        name: "af_nicole",
    },
    Voice {
        sid: 7,
        name: "af_nova",
    },
    Voice {
        sid: 8,
        name: "af_river",
    },
    Voice {
        sid: 9,
        name: "af_sarah",
    },
    Voice {
        sid: 10,
        name: "af_sky",
    },
    Voice {
        sid: 11,
        name: "am_adam",
    },
    Voice {
        sid: 12,
        name: "am_echo",
    },
    Voice {
        sid: 13,
        name: "am_eric",
    },
    Voice {
        sid: 14,
        name: "am_fenrir",
    },
    Voice {
        sid: 15,
        name: "am_liam",
    },
    Voice {
        sid: 16,
        name: "am_michael",
    },
    Voice {
        sid: 17,
        name: "am_onyx",
    },
    Voice {
        sid: 18,
        name: "am_puck",
    },
    Voice {
        sid: 19,
        name: "am_santa",
    },
    Voice {
        sid: 20,
        name: "bf_alice",
    },
    Voice {
        sid: 21,
        name: "bf_emma",
    },
    Voice {
        sid: 22,
        name: "bf_isabella",
    },
    Voice {
        sid: 23,
        name: "bf_lily",
    },
    Voice {
        sid: 24,
        name: "bm_daniel",
    },
    Voice {
        sid: 25,
        name: "bm_fable",
    },
    Voice {
        sid: 26,
        name: "bm_george",
    },
    Voice {
        sid: 27,
        name: "bm_lewis",
    },
    Voice {
        sid: 28,
        name: "ef_dora",
    },
    Voice {
        sid: 29,
        name: "em_alex",
    },
    Voice {
        sid: 30,
        name: "ff_siwis",
    },
    Voice {
        sid: 31,
        name: "hf_alpha",
    },
    Voice {
        sid: 32,
        name: "hf_beta",
    },
    Voice {
        sid: 33,
        name: "hm_omega",
    },
    Voice {
        sid: 34,
        name: "hm_psi",
    },
    Voice {
        sid: 35,
        name: "if_sara",
    },
    Voice {
        sid: 36,
        name: "im_nicola",
    },
    Voice {
        sid: 37,
        name: "jf_alpha",
    },
    Voice {
        sid: 38,
        name: "jf_gongitsune",
    },
    Voice {
        sid: 39,
        name: "jf_nezumi",
    },
    Voice {
        sid: 40,
        name: "jf_tebukuro",
    },
    Voice {
        sid: 41,
        name: "jm_kumo",
    },
    Voice {
        sid: 42,
        name: "pf_dora",
    },
    Voice {
        sid: 43,
        name: "pm_alex",
    },
    Voice {
        sid: 44,
        name: "pm_santa",
    },
    Voice {
        sid: 45,
        name: "zf_xiaobei",
    },
    Voice {
        sid: 46,
        name: "zf_xiaoni",
    },
    Voice {
        sid: 47,
        name: "zf_xiaoxiao",
    },
    Voice {
        sid: 48,
        name: "zf_xiaoyi",
    },
    Voice {
        sid: 49,
        name: "zm_yunjian",
    },
    Voice {
        sid: 50,
        name: "zm_yunxi",
    },
    Voice {
        sid: 51,
        name: "zm_yunxia",
    },
    Voice {
        sid: 52,
        name: "zm_yunyang",
    },
    Voice {
        sid: 53,
        name: "em_santa",
    },
];

/// `kokoro-multi-lang-v1_1`: 103 speakers, Chinese + English.
pub const V1_1: &[Voice] = &[
    Voice {
        sid: 0,
        name: "af_maple",
    },
    Voice {
        sid: 1,
        name: "af_sol",
    },
    Voice {
        sid: 2,
        name: "bf_vale",
    },
    Voice {
        sid: 3,
        name: "zf_001",
    },
    Voice {
        sid: 4,
        name: "zf_002",
    },
    Voice {
        sid: 5,
        name: "zf_003",
    },
    Voice {
        sid: 6,
        name: "zf_004",
    },
    Voice {
        sid: 7,
        name: "zf_005",
    },
    Voice {
        sid: 8,
        name: "zf_006",
    },
    Voice {
        sid: 9,
        name: "zf_007",
    },
    Voice {
        sid: 10,
        name: "zf_008",
    },
    Voice {
        sid: 11,
        name: "zf_017",
    },
    Voice {
        sid: 12,
        name: "zf_018",
    },
    Voice {
        sid: 13,
        name: "zf_019",
    },
    Voice {
        sid: 14,
        name: "zf_021",
    },
    Voice {
        sid: 15,
        name: "zf_022",
    },
    Voice {
        sid: 16,
        name: "zf_023",
    },
    Voice {
        sid: 17,
        name: "zf_024",
    },
    Voice {
        sid: 18,
        name: "zf_026",
    },
    Voice {
        sid: 19,
        name: "zf_027",
    },
    Voice {
        sid: 20,
        name: "zf_028",
    },
    Voice {
        sid: 21,
        name: "zf_032",
    },
    Voice {
        sid: 22,
        name: "zf_036",
    },
    Voice {
        sid: 23,
        name: "zf_038",
    },
    Voice {
        sid: 24,
        name: "zf_039",
    },
    Voice {
        sid: 25,
        name: "zf_040",
    },
    Voice {
        sid: 26,
        name: "zf_042",
    },
    Voice {
        sid: 27,
        name: "zf_043",
    },
    Voice {
        sid: 28,
        name: "zf_044",
    },
    Voice {
        sid: 29,
        name: "zf_046",
    },
    Voice {
        sid: 30,
        name: "zf_047",
    },
    Voice {
        sid: 31,
        name: "zf_048",
    },
    Voice {
        sid: 32,
        name: "zf_049",
    },
    Voice {
        sid: 33,
        name: "zf_051",
    },
    Voice {
        sid: 34,
        name: "zf_059",
    },
    Voice {
        sid: 35,
        name: "zf_060",
    },
    Voice {
        sid: 36,
        name: "zf_067",
    },
    Voice {
        sid: 37,
        name: "zf_070",
    },
    Voice {
        sid: 38,
        name: "zf_071",
    },
    Voice {
        sid: 39,
        name: "zf_072",
    },
    Voice {
        sid: 40,
        name: "zf_073",
    },
    Voice {
        sid: 41,
        name: "zf_074",
    },
    Voice {
        sid: 42,
        name: "zf_075",
    },
    Voice {
        sid: 43,
        name: "zf_076",
    },
    Voice {
        sid: 44,
        name: "zf_077",
    },
    Voice {
        sid: 45,
        name: "zf_078",
    },
    Voice {
        sid: 46,
        name: "zf_079",
    },
    Voice {
        sid: 47,
        name: "zf_083",
    },
    Voice {
        sid: 48,
        name: "zf_084",
    },
    Voice {
        sid: 49,
        name: "zf_085",
    },
    Voice {
        sid: 50,
        name: "zf_086",
    },
    Voice {
        sid: 51,
        name: "zf_087",
    },
    Voice {
        sid: 52,
        name: "zf_088",
    },
    Voice {
        sid: 53,
        name: "zf_090",
    },
    Voice {
        sid: 54,
        name: "zf_092",
    },
    Voice {
        sid: 55,
        name: "zf_093",
    },
    Voice {
        sid: 56,
        name: "zf_094",
    },
    Voice {
        sid: 57,
        name: "zf_099",
    },
    Voice {
        sid: 58,
        name: "zm_009",
    },
    Voice {
        sid: 59,
        name: "zm_010",
    },
    Voice {
        sid: 60,
        name: "zm_011",
    },
    Voice {
        sid: 61,
        name: "zm_012",
    },
    Voice {
        sid: 62,
        name: "zm_013",
    },
    Voice {
        sid: 63,
        name: "zm_014",
    },
    Voice {
        sid: 64,
        name: "zm_015",
    },
    Voice {
        sid: 65,
        name: "zm_016",
    },
    Voice {
        sid: 66,
        name: "zm_020",
    },
    Voice {
        sid: 67,
        name: "zm_025",
    },
    Voice {
        sid: 68,
        name: "zm_029",
    },
    Voice {
        sid: 69,
        name: "zm_030",
    },
    Voice {
        sid: 70,
        name: "zm_031",
    },
    Voice {
        sid: 71,
        name: "zm_033",
    },
    Voice {
        sid: 72,
        name: "zm_034",
    },
    Voice {
        sid: 73,
        name: "zm_035",
    },
    Voice {
        sid: 74,
        name: "zm_037",
    },
    Voice {
        sid: 75,
        name: "zm_041",
    },
    Voice {
        sid: 76,
        name: "zm_045",
    },
    Voice {
        sid: 77,
        name: "zm_050",
    },
    Voice {
        sid: 78,
        name: "zm_052",
    },
    Voice {
        sid: 79,
        name: "zm_053",
    },
    Voice {
        sid: 80,
        name: "zm_054",
    },
    Voice {
        sid: 81,
        name: "zm_055",
    },
    Voice {
        sid: 82,
        name: "zm_056",
    },
    Voice {
        sid: 83,
        name: "zm_057",
    },
    Voice {
        sid: 84,
        name: "zm_058",
    },
    Voice {
        sid: 85,
        name: "zm_061",
    },
    Voice {
        sid: 86,
        name: "zm_062",
    },
    Voice {
        sid: 87,
        name: "zm_063",
    },
    Voice {
        sid: 88,
        name: "zm_064",
    },
    Voice {
        sid: 89,
        name: "zm_065",
    },
    Voice {
        sid: 90,
        name: "zm_066",
    },
    Voice {
        sid: 91,
        name: "zm_068",
    },
    Voice {
        sid: 92,
        name: "zm_069",
    },
    Voice {
        sid: 93,
        name: "zm_080",
    },
    Voice {
        sid: 94,
        name: "zm_081",
    },
    Voice {
        sid: 95,
        name: "zm_082",
    },
    Voice {
        sid: 96,
        name: "zm_089",
    },
    Voice {
        sid: 97,
        name: "zm_091",
    },
    Voice {
        sid: 98,
        name: "zm_095",
    },
    Voice {
        sid: 99,
        name: "zm_096",
    },
    Voice {
        sid: 100,
        name: "zm_097",
    },
    Voice {
        sid: 101,
        name: "zm_098",
    },
    Voice {
        sid: 102,
        name: "zm_100",
    },
];

/// The catalog for a model reporting `num_speakers` speakers.
///
/// Empty when the model is not one of the bundles listed above: the ids are
/// still usable directly through [`SpeakConfig::voice`], only the names are
/// unknown.
///
/// [`SpeakConfig::voice`]: super::SpeakConfig::voice
#[must_use]
pub fn catalog(num_speakers: u32) -> &'static [Voice] {
    match num_speakers {
        // The v1.0 documentation counts 53 speakers but publishes ids 0-53
        // (54 entries), so accept both counts.
        53 | 54 => V1_0,
        103 => V1_1,
        _ => &[],
    }
}

/// Resolve a published voice name against both bundles.
#[must_use]
pub fn find(name: &str) -> Option<Voice> {
    V1_1.iter()
        .chain(V1_0)
        .copied()
        .find(|voice| voice.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_well_formed(table: &[Voice], expected: usize) {
        assert_eq!(table.len(), expected);
        for (index, voice) in table.iter().enumerate() {
            assert_eq!(voice.sid, index as i32, "ids must be 0..n in order");
            assert!(!voice.name.is_empty());
        }
        let mut names: Vec<_> = table.iter().map(|voice| voice.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), table.len(), "voice names must be unique");
    }

    #[test]
    fn catalogs_are_complete_and_dense() {
        assert_well_formed(V1_0, 54);
        assert_well_formed(V1_1, 103);
        assert_eq!(catalog(54).len(), 54);
        assert_eq!(catalog(103).len(), 103);
        assert!(catalog(7).is_empty());
    }

    #[test]
    fn names_resolve_to_the_published_ids() {
        assert_eq!(
            find("af_maple"),
            Some(Voice {
                sid: 0,
                name: "af_maple"
            })
        );
        assert_eq!(
            find("zm_100"),
            Some(Voice {
                sid: 102,
                name: "zm_100"
            })
        );
        assert_eq!(
            find("af_heart"),
            Some(Voice {
                sid: 3,
                name: "af_heart"
            }),
            "v1.0 names stay resolvable"
        );
        assert_eq!(
            find("pm_alex"),
            Some(Voice {
                sid: 43,
                name: "pm_alex"
            })
        );
        assert_eq!(find("not_a_voice"), None);
    }
}
