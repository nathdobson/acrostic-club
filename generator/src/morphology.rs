use std::io;
use crate::PACKAGE_PATH;

#[derive(Debug)]
pub struct MorphSet {
    entries: Vec<MorphEntry>,
}

#[derive(Debug, Clone, Copy)]
enum MorphType {
    N,
    Ns,
    V0,
    Vs,
    Ved,
    Ven,
    Ving,
}

#[derive(Debug)]
pub struct MorphEntry {
    word: String,
    root: String,
    typ: MorphType,
}

impl MorphSet {
    pub async fn new() -> io::Result<Self> {
        let mut entries = vec![];
        let content = tokio::fs::read_to_string(PACKAGE_PATH.join("submodules/catvar/English-Morph.txt")).await?;
        for line in content.split("\n") {
            if line.is_empty() {
                continue;
            }
            let mut cells = line.split("\t").collect::<Vec<_>>();
            let typ = match cells.pop().unwrap() {
                "$N$" => MorphType::N,
                "$N+s$" => MorphType::Ns,
                "$V+0$" => MorphType::V0,
                "$V+s$" => MorphType::Vs,
                "$V+ed$" => MorphType::Ved,
                "$V+en$" => MorphType::Ven,
                "$V+ing$" => MorphType::Ving,
                "$V+s/neg$" => MorphType::Vs,//
                "$V+ed/neg$" => MorphType::Ved,//
                "$V+0/1ppl$" => MorphType::V0,//
                "" => MorphType::Vs,//
                x => panic!("{:?}", x),
            };
            let mut root = cells.pop().unwrap();
            for word in cells {
                entries.push(MorphEntry {
                    word: word.to_string(),
                    root: root.to_string(),
                    typ,
                });
            }
        }
        Ok(MorphSet { entries })
    }
}

#[tokio::test]
async fn test_morph() -> io::Result<()> {
    let morph_set = MorphSet::new().await?;
    println!("{:?}", morph_set);
    Ok(())
}