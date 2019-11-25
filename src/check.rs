use quote::quote;
use syn::{Arm, Attribute, Ident, Result, Variant};
use syn::{Error, Field, Pat, PatIdent};

use crate::compare::Path;
use crate::format;
use crate::parse::Input::{self, *};

pub fn sorted(input: &mut Input) -> Result<()> {
    let paths = match input {
        Enum(item) => collect_paths(&mut item.variants)?,
        Struct(item) => collect_paths(&mut item.fields)?,
        Match(expr) | Let(expr) => collect_paths(&mut expr.arms)?,
    };

    for i in 1..paths.len() {
        let cur = &paths[i];
        if *cur < paths[i - 1] {
            let lesser = cur;
            let correct_pos = paths[..i - 1].binary_search(cur).unwrap_err();
            let greater = &paths[correct_pos];
            return Err(format::error(lesser, greater));
        }
    }

    Ok(())
}

fn collect_paths<'a, I, P>(iter: I) -> Result<Vec<Path>>
where
    I: IntoIterator<Item = &'a mut P>,
    P: Sortable + 'a,
{
    iter.into_iter()
        .filter_map(|item| {
            if remove_unsorted_attr(item.attrs()) {
                None
            } else {
                Some(item.to_path())
            }
        })
        .collect()
}

fn remove_unsorted_attr(attrs: &mut Vec<Attribute>) -> bool {
    for i in 0..attrs.len() {
        let path = &attrs[i].path;
        let path = quote!(#path).to_string();
        if path == "unsorted" || path == "remain :: unsorted" {
            attrs.remove(i);
            return true;
        }
    }

    false
}

trait Sortable {
    fn to_path(&self) -> Result<Path>;
    fn attrs(&mut self) -> &mut Vec<Attribute>;
}

impl Sortable for Variant {
    fn to_path(&self) -> Result<Path> {
        Ok(Path {
            segments: vec![self.ident.clone()],
        })
    }
    fn attrs(&mut self) -> &mut Vec<Attribute> {
        &mut self.attrs
    }
}

impl Sortable for Field {
    fn to_path(&self) -> Result<Path> {
        Ok(Path {
            segments: vec![self.ident.clone().expect("must be named field")],
        })
    }
    fn attrs(&mut self) -> &mut Vec<Attribute> {
        &mut self.attrs
    }
}

impl Sortable for Arm {
    fn to_path(&self) -> Result<Path> {
        // Sort by just the first pat.
        let pat = match &self.pat {
            Pat::Or(pat) => pat.cases.iter().next().expect("at least one pat"),
            _ => &self.pat,
        };

        let segments = match pat {
            Pat::Ident(pat) if is_just_ident(&pat) => vec![pat.ident.clone()],
            Pat::Path(pat) => idents_of_path(&pat.path),
            Pat::Struct(pat) => idents_of_path(&pat.path),
            Pat::TupleStruct(pat) => idents_of_path(&pat.path),
            Pat::Wild(pat) => vec![Ident::from(pat.underscore_token)],
            other => {
                let msg = "unsupported by #[remain::sorted]";
                return Err(Error::new_spanned(other, msg));
            }
        };

        Ok(Path { segments })
    }
    fn attrs(&mut self) -> &mut Vec<Attribute> {
        &mut self.attrs
    }
}

fn idents_of_path(path: &syn::Path) -> Vec<Ident> {
    path.segments
        .clone()
        .into_iter()
        .map(|seg| seg.ident)
        .collect()
}

fn is_just_ident(pat: &PatIdent) -> bool {
    pat.by_ref.is_none() && pat.mutability.is_none() && pat.subpat.is_none()
}
