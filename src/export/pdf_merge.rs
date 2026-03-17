//! PDF merge utility.
//!
//! lopdf doesn't provide a document-level merge API.
//! We implement page-by-page copying with object ID remapping.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use lopdf::{dictionary, Document, Object, ObjectId};

/// Merge multiple PDF files into a single output file.
pub fn merge_pdfs(input_paths: &[impl AsRef<Path>], output_path: &Path) -> Result<()> {
    if input_paths.is_empty() {
        anyhow::bail!("No PDFs to merge");
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }

    // If only one PDF, just copy it
    if input_paths.len() == 1 {
        std::fs::copy(input_paths[0].as_ref(), output_path)
            .context("Failed to copy single PDF")?;
        return Ok(());
    }

    // Collect all documents
    let mut documents: Vec<Document> = Vec::new();
    for path in input_paths {
        let path = path.as_ref();
        let doc = Document::load(path)
            .with_context(|| format!("Failed to load PDF: {}", path.display()))?;
        documents.push(doc);
    }

    // Build merged document: copy all objects with remapped IDs, collect page references
    let mut merged_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut page_refs: Vec<ObjectId> = Vec::new();
    let mut next_id: u32 = 1;

    for doc in &documents {
        // Build ID remap table for this document
        let mut id_map: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
        for &old_id in doc.objects.keys() {
            let new_id = (next_id, old_id.1); // preserve generation number
            id_map.insert(old_id, new_id);
            next_id += 1;
        }

        // Copy objects with remapped references
        for (old_id, obj) in &doc.objects {
            let new_id = id_map[old_id];
            let remapped_obj = remap_object_refs(obj.clone(), &id_map);
            merged_objects.insert(new_id, remapped_obj);
        }

        // Collect remapped page object IDs
        let pages = doc.get_pages();
        let mut page_nums: Vec<u32> = pages.keys().copied().collect();
        page_nums.sort();
        for page_num in page_nums {
            if let Some(&old_page_id) = pages.get(&page_num) {
                if let Some(&new_page_id) = id_map.get(&old_page_id) {
                    page_refs.push(new_page_id);
                }
            }
        }
    }

    // Build a new document with a fresh catalog and page tree
    let mut merged = Document::with_version("1.5");

    // Insert all copied objects
    for (id, obj) in merged_objects {
        merged.objects.insert(id, obj);
    }

    // Create Pages dictionary (page tree root)
    let pages_id = (next_id, 0);
    next_id += 1;

    let kids: Vec<Object> = page_refs
        .iter()
        .map(|&id| Object::Reference(id))
        .collect();

    // Update each page's Parent reference to point to our new Pages node
    for &page_id in &page_refs {
        if let Some(Object::Dictionary(ref mut dict)) = merged.objects.get_mut(&page_id) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Count" => Object::Integer(page_refs.len() as i64),
        "Kids" => Object::Array(kids),
    };
    merged.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Create Catalog
    let catalog_id = (next_id, 0);
    let catalog_dict = dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    merged.objects.insert(catalog_id, Object::Dictionary(catalog_dict));

    merged.trailer.set("Root", Object::Reference(catalog_id));

    // Renumber and save
    merged.renumber_objects();
    merged.compress();
    merged.save(output_path)
        .with_context(|| format!("Failed to save merged PDF: {}", output_path.display()))?;

    Ok(())
}

/// Recursively remap object ID references within an object tree.
fn remap_object_refs(obj: Object, id_map: &BTreeMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => {
            Object::Reference(*id_map.get(&id).unwrap_or(&id))
        }
        Object::Array(arr) => {
            Object::Array(arr.into_iter().map(|o| remap_object_refs(o, id_map)).collect())
        }
        Object::Dictionary(mut dict) => {
            // Remove Parent references from page dicts to avoid circular refs;
            // we'll set them correctly later
            let keys: Vec<Vec<u8>> = dict.as_hashmap().keys().cloned().collect();
            for key in keys {
                if let Ok(val) = dict.get(&key) {
                    let remapped = remap_object_refs(val.clone(), id_map);
                    dict.set(key, remapped);
                }
            }
            Object::Dictionary(dict)
        }
        Object::Stream(mut stream) => {
            let keys: Vec<Vec<u8>> = stream.dict.as_hashmap().keys().cloned().collect();
            for key in keys {
                if let Ok(val) = stream.dict.get(&key) {
                    let remapped = remap_object_refs(val.clone(), id_map);
                    stream.dict.set(key, remapped);
                }
            }
            Object::Stream(stream)
        }
        other => other,
    }
}
