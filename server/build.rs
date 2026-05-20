use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Deserialize)]
struct RawData {
    blocks: HashMap<String, RawBlock>,
}

#[derive(Deserialize)]
struct RawBlock {
    default_state_id: u16,
    properties: HashMap<String, Vec<String>>,
    states: Vec<RawState>,
}

#[derive(Deserialize)]
struct RawState {
    id: u16,
    properties: HashMap<String, String>,
}

fn main() {
    println!("cargo:rerun-if-changed=data/block_states.json");

    let json_path = Path::new("data/block_states.json");
    let json_str = fs::read_to_string(json_path).expect("Failed to read data/block_states.json");
    let raw: RawData = serde_json::from_str(&json_str).expect("Failed to parse block_states.json");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("block_states_generated.rs");
    let file = fs::File::create(&dest_path).unwrap();
    let mut w = BufWriter::new(file);

    // Sort blocks by name for deterministic output
    let mut blocks: Vec<_> = raw.blocks.into_iter().collect();
    blocks.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate BlockDef and StateDef structs
    writeln!(w, "pub struct BlockDef {{").unwrap();
    writeln!(w, "    pub name: &'static str,").unwrap();
    writeln!(w, "    pub default_state_id: u16,").unwrap();
    writeln!(
        w,
        "    pub properties: &'static [(&'static str, &'static [&'static str])],"
    )
    .unwrap();
    writeln!(w, "    pub states: &'static [StateDef],").unwrap();
    writeln!(w, "}}").unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub struct StateDef {{").unwrap();
    writeln!(w, "    pub id: u16,").unwrap();
    writeln!(
        w,
        "    pub properties: &'static [(&'static str, &'static str)],"
    )
    .unwrap();
    writeln!(w, "}}").unwrap();
    writeln!(w).unwrap();

    // Generate static data for each block
    for (i, (name, block)) in blocks.iter().enumerate() {
        // Sort properties by key for deterministic output
        let mut props: Vec<_> = block.properties.iter().collect();
        props.sort_by_key(|(k, _)| *k);

        // Generate property values arrays
        for (pi, (_prop_name, values)) in props.iter().enumerate() {
            write!(w, "static BLOCK_{i}_PROP_{pi}_VALUES: &[&str] = &[",).unwrap();
            for v in values.iter() {
                write!(w, "\"{v}\", ").unwrap();
            }
            writeln!(w, "];").unwrap();
        }

        // Generate properties array
        writeln!(w, "static BLOCK_{i}_PROPERTIES: &[(&str, &[&str])] = &[",).unwrap();
        for (pi, (prop_name, _)) in props.iter().enumerate() {
            writeln!(w, "    (\"{prop_name}\", BLOCK_{i}_PROP_{pi}_VALUES),").unwrap();
        }
        writeln!(w, "];").unwrap();

        // Generate state entries
        // Sort states by id for deterministic output
        let mut states: Vec<_> = block.states.iter().collect();
        states.sort_by_key(|s| s.id);

        for (si, state) in states.iter().enumerate() {
            let mut state_props: Vec<_> = state.properties.iter().collect();
            state_props.sort_by_key(|(k, _)| *k);

            write!(w, "static BLOCK_{i}_STATE_{si}_PROPS: &[(&str, &str)] = &[",).unwrap();
            for (k, v) in &state_props {
                write!(w, "(\"{k}\", \"{v}\"), ").unwrap();
            }
            writeln!(w, "];").unwrap();
        }

        writeln!(w, "static BLOCK_{i}_STATES: &[StateDef] = &[").unwrap();
        for (si, state) in states.iter().enumerate() {
            writeln!(
                w,
                "    StateDef {{ id: {}, properties: BLOCK_{i}_STATE_{si}_PROPS }},",
                state.id
            )
            .unwrap();
        }
        writeln!(w, "];").unwrap();

        // Generate BlockDef
        writeln!(w, "static BLOCK_{i}_DEF: BlockDef = BlockDef {{",).unwrap();
        writeln!(w, "    name: \"{name}\",").unwrap();
        writeln!(w, "    default_state_id: {},", block.default_state_id).unwrap();
        writeln!(w, "    properties: BLOCK_{i}_PROPERTIES,").unwrap();
        writeln!(w, "    states: BLOCK_{i}_STATES,").unwrap();
        writeln!(w, "}};").unwrap();
        writeln!(w).unwrap();
    }

    // Generate BLOCKS phf map (name -> &BlockDef)
    write!(
        w,
        "pub static BLOCKS: phf::Map<&'static str, &'static BlockDef> = "
    )
    .unwrap();
    let mut blocks_map = phf_codegen::Map::new();
    for (i, (name, _)) in blocks.iter().enumerate() {
        blocks_map.entry(name.as_str(), &format!("&BLOCK_{i}_DEF"));
    }
    write!(w, "{}", blocks_map.build()).unwrap();
    writeln!(w, ";").unwrap();
    writeln!(w).unwrap();

    // Generate STATES phf map (state_id -> (&'static str, &'static [(&str, &str)]))
    // We store (block_name, state_properties) for reverse lookup
    write!(
        w,
        "pub static STATES: phf::Map<u16, (&'static str, &'static [(&'static str, &'static str)])> = "
    )
    .unwrap();
    let mut states_map = phf_codegen::Map::new();
    for (i, (name, block)) in blocks.iter().enumerate() {
        let mut states: Vec<_> = block.states.iter().collect();
        states.sort_by_key(|s| s.id);
        for (si, state) in states.iter().enumerate() {
            states_map.entry(
                state.id,
                &format!("(\"{name}\", BLOCK_{i}_STATE_{si}_PROPS)"),
            );
        }
    }
    write!(w, "{}", states_map.build()).unwrap();
    writeln!(w, ";").unwrap();
    writeln!(w).unwrap();

    // Generate block count constant
    writeln!(w, "pub const BLOCK_COUNT: usize = {};", blocks.len()).unwrap();
}
