use defs::{CrateSaveAnalysis, CrateDisambiguator,
	CrateSaveAnalysisMetadata};
use StrErr;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};

use defs::{Def, Ref, ItemId, Prelude};

pub type AbsItemId = ItemId<CrateDisambiguator>;

pub type AbsDef = Def<CrateDisambiguator>;
pub type AbsRef = Ref<CrateDisambiguator>;

pub struct AnalysisDb {
	root :Option<PathBuf>,
	covered_crates :HashSet<CrateDisambiguator>,
	defs :HashMap<AbsItemId, AbsDef>,
	refs :HashMap<AbsItemId, AbsRef>,
}

impl<T> ItemId<T> {
	fn clone_map<U>(&self, f :impl FnOnce(&T) -> U) -> ItemId<U> {
		ItemId {
			krate : f(&self.krate),
			index : self.index,
		}
	}
}

impl<T> Def<T> {
	fn clone_map<U>(&self, f :impl Fn(&T) -> U) -> Def<U> {
		Def {
			kind : self.kind.clone(),
			name : self.name.clone(),
			id : self.id.clone_map(&f),
			span : self.span.clone(),
			parent : self.parent.as_ref().map(|v| v.clone_map(&f)),
			decl_id : self.decl_id.as_ref().map(|v| v.clone_map(&f)),
		}
	}

	fn is_in_macro<'a>(&self, prefix :impl Into<&'a Path>) -> Result<bool, StrErr> {
		use syn::parse::Parser;
		use syn::parse::ParseStream;
		use syn::{Attribute, Item, Macro};
		use syn::spanned::Spanned;
		use syn::visit::visit_item;
		let mut path = prefix.into().to_owned();
		path.push(&self.span.file_name);
		let file = std::fs::read_to_string(path)?;
		struct Visitor {
			found :bool,
			needle_span :crate::defs::Span,
		}
		impl<'ast> syn::visit::Visit<'ast> for Visitor {
			fn visit_macro(&mut self, m :&'ast Macro) {
				let sp = m.span();
				// TODO this code is broken atm as the span is only
				// the span of the macro name.
				if (sp.start().line, sp.start().column) <= (self.needle_span.line_start as usize, self.needle_span.column_start as usize)
						&& (sp.end().line, sp.end().column) >= (self.needle_span.line_end as usize, self.needle_span.column_end as usize) {
					self.found = true;
				}
			}
		}
		let (_attrs, items) = (|stream :ParseStream| {
			let attrs = stream.call(Attribute::parse_inner)?;
			let mut items = Vec::new();
			while !stream.is_empty() {
				let item :Item = stream.parse()?;
				items.push(item);
			}
			Ok((attrs, items))
		}).parse_str(&file)?;


		for item in items.iter() {
			let mut visitor = Visitor {
				found : false,
				needle_span : self.span.clone(),
			};
			visit_item(&mut visitor, &item);
			if visitor.found {
				return Ok(true)
			}
		}
		Ok(false)
	}
}

impl<T> Ref<T> {
	fn clone_map<U>(&self, f :impl FnOnce(&T) -> U) -> Ref<U> {
		Ref {
			kind : self.kind.clone(),
			ref_id : ItemId {
				krate : f(&self.ref_id.krate),
				index : self.ref_id.index,
			},
			span : self.span.clone(),
		}
	}
}

fn parse_save_analysis(path :&Path) -> Result<CrateSaveAnalysis, StrErr> {
	let file = std::fs::read_to_string(path)?;
	let file_parsed :CrateSaveAnalysis = serde_json::from_str(&file)?;
	Ok(file_parsed)
}
fn parse_analysis_metadata(path :&Path) -> Result<CrateSaveAnalysisMetadata, StrErr> {
	let file = std::fs::read_to_string(path)?;
	let file_parsed :CrateSaveAnalysisMetadata = serde_json::from_str(&file)?;
	Ok(file_parsed)
}

impl Prelude {
	fn disambiguator_for_id(&self, id :u32) -> CrateDisambiguator {
		if id == 0 {
			return self.crate_id.disambiguator;
		}
		let krate = &self.external_crates[(id - 1) as usize];
		assert_eq!(krate.num, id);
		krate.id.disambiguator
	}
}

impl AnalysisDb {
	pub fn from_path(path :&str) -> Result<Self, StrErr> {
		let path = Path::new(path);
		let leaf_parsed = parse_analysis_metadata(&path)?;
		let mut disambiguators = leaf_parsed.prelude.external_crates.iter()
			.map(|v| v.id.disambiguator)
			.collect::<HashSet<_>>();
		disambiguators.insert(leaf_parsed.prelude.crate_id.disambiguator);
		let dir_path = path.parent().unwrap();
		let mut crates = HashMap::new();
		let mut covered_crates = HashSet::new();
		for entry in std::fs::read_dir(dir_path)? {
			let entry = entry?;
			let path = entry.path();
			let metadata = parse_analysis_metadata(&path)?;
			let disambiguator = metadata.prelude.crate_id.disambiguator;
			// Ignore results from other compile runs
			if !disambiguators.contains(&disambiguator) {
				continue;
			}

			// Ignore stuff from crates.io or git deps.
			// Just focus on path deps for now.
			if metadata.compilation.directory.contains(".cargo/registry/src/github.com") ||
					metadata.compilation.directory.contains(".cargo/git/") {
				println!("i> {}", path.to_str().unwrap());
				continue;
			}
			println!("p> {}", path.to_str().unwrap());
			let file_parsed = parse_save_analysis(&path)?;
			crates.insert(disambiguator, file_parsed);
			covered_crates.insert(disambiguator);
		}
		let mut defs = HashMap::new();
		for (_dis, c) in crates.iter() {
			for v in c.defs.iter() {
				let v = v.clone_map(|w| c.prelude.disambiguator_for_id(*w));
				defs.insert(v.id, v);
			}
		}
		let mut refs = HashMap::new();
		for (_dis, c) in crates.iter() {
			for v in c.refs.iter() {
				let v = v.clone_map(|w| c.prelude.disambiguator_for_id(*w));
				refs.insert(v.ref_id, v);
			}
		}
		//println!("{:#?}", defs);
		//println!("{:#?}", refs);

		let root = path.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.map(|p| p.to_owned());
		Ok(AnalysisDb {
			root,
			covered_crates,
			defs,
			refs,
		})
	}
	pub fn get_unused_defs(&self) -> impl Iterator<Item=&AbsDef> {
		let mut used_defs = HashSet::new();
		for (_rid, r) in self.refs.iter() {
			used_defs.insert(r.ref_id);
		}
		let mut unused_defs = Vec::new();
		for (did, d) in self.defs.iter() {
			if used_defs.contains(&did) {
				continue;
			}
			// Anything starting with _ can be unused without warning.
			if d.name.starts_with("_") {
				continue;
			}
			// Self may be unused without warning.
			if d.kind == "Local" && d.name == "self" {
				continue;
			}
			// There is an id mismatch bug in rustc's save-analysis
			// output.
			// https://github.com/rust-lang/rust/issues/61302
			if d.kind == "TupleVariant" {
				continue;
			}
			// Record implementations of traits etc as used if the trait's
			// function is used
			if let Some(decl_id) = d.decl_id {
				// Whether the trait's fn is used somewhere
				let fn_in_trait_used = used_defs.contains(&decl_id);
				// Whether the trait is from another crate
				let fn_in_trait_foreign = !self.covered_crates.contains(&decl_id.krate);
				if fn_in_trait_used || fn_in_trait_foreign {
					continue;
				}
			}
			if let Some(parent) = d.parent.as_ref().and_then(|p| self.defs.get(p)) {
				// It seems that rustc doesn't emit any refs for assoc. types
				if parent.kind == "Trait" && d.kind == "Type" {
					continue;
				}
			}
			// Macros have poor save-analysis support atm:
			// https://github.com/rust-lang/rust/issues/49178#issuecomment-375454487
			// Most importantly, their spans are not emitted.
			let root = self.root.clone().unwrap_or_else(PathBuf::new);
			if d.is_in_macro(root.as_path()).unwrap_or(false) {
				continue;
			}
			//std::fs::read_to_string d.span.file_name
			unused_defs.push(d);
		}
		unused_defs.sort();
		unused_defs.into_iter()
	}
}
