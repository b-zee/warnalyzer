use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct CrateDisambiguator(pub u64, pub u64);

#[derive(Deserialize, Debug)]
pub struct CrateId {
	pub name :String,
	pub disambiguator :CrateDisambiguator,
}

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct ItemId<KrateId> {
	pub krate :KrateId,
	pub index :u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Span {
	pub file_name :String,
	pub line_start :u32,
	pub line_end :u32,
	pub column_start :u32,
	pub column_end :u32,
}

#[derive(Deserialize, Debug)]
pub struct ExternalCrate {
	pub num :u32,
	pub id :CrateId,
}

#[derive(Deserialize, Debug)]
pub struct Prelude {
	pub crate_id :CrateId,
	pub external_crates :Vec<ExternalCrate>,
}

#[derive(Deserialize, Debug)]
pub struct Def<KrateId> {
	pub kind :String,
	pub name :String,
	pub id :ItemId<KrateId>,
	pub span :Span,
}

#[derive(Deserialize, Debug)]
pub struct Ref<KrateId> {
	pub kind :String,
	pub ref_id :ItemId<KrateId>,
	pub span :Span,
}

#[derive(Deserialize, Debug)]
pub struct CrateSaveAnalysis {
	pub prelude :Prelude,
	pub defs :Vec<Def<u32>>,
	pub refs :Vec<Ref<u32>>,
}
