use crate::Data;
use dapp_lib::prelude::AppType;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::{DefaultHasher, Hasher};

#[derive(Clone)]
pub struct Manifest {
    pub app_type: AppType,
    pub description: String,
    pub tags: HashMap<u8, Tag>,
    pub d_types: HashMap<u8, Tag>,
}
// TODO: a new Manifest definition, with attributes being added as needed during development
//  Manifest should apply to a Swarm, not an Application, application is defined in code
//  and can support different kinds of Swarms distinguished by their AppType
//  Manifest defines application type, tags, data structures, and message headers:
//  It consist of two elements: a 495-byte long header, and a HashMap<u8,u16>.
//  Header contains a general application description.
//  Mapping stores partial ContentIDs of locally stored Content that holds further
//  definitions required for application to function.
//  Those ContentIDs should all have the same Datatype value = 255.
//  Header may contain instructions on how to decrypt those Contents.
//  There can be up to 256 tags defined for a given swarm.
//  There can be up to 256 top level data structures defined in a single application.
//  There can be up to 256 top level synchronization messages defined.
//  There can be also some (less than 256) top level reconfiguration messages defined.
//  (We already have some built-in Reconfigs.)

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag(pub String);
impl Tag {
    pub fn new(name: String) -> Result<Self, ()> {
        if name.len() <= 32 {
            Ok(Tag(name))
        } else {
            Err(())
        }
    }
    pub fn bytes(&self) -> Vec<u8> {
        let len = self.0.len();
        let mut bytes = Vec::with_capacity(32);
        for _i in len..32 {
            bytes.push(32); //Fill with ' ' char
        }
        for byte in self.0.bytes() {
            bytes.push(byte);
        }
        bytes
    }
}
impl Manifest {
    pub fn new(app_type: AppType, tags: HashMap<u8, Tag>) -> Self {
        Manifest {
            app_type,
            description: String::new(),
            tags,
            d_types: HashMap::new(),
        }
    }

    pub fn set_description(&mut self, text: String) -> bool {
        if text.len() > 1022 {
            return false;
        }
        self.description = text;
        true
    }

    // first Data consists only of:
    // - app_type
    // - tags count
    // - description
    pub fn first_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let mut bytes = Vec::with_capacity(1024);
        bytes.push(self.app_type.byte());
        bytes.push(self.tags.len() as u8);
        //
        for byte in self.description.bytes() {
            bytes.push(byte);
        }

        // for i in 0..=31 {
        //     // bytes.push(i);
        //     if let Some(tag_name) = self.tags.get(&i) {
        //         // let tag_len = tag_name.0.len() as u8;
        //         // bytes.push(i);
        //         // bytes.push(tag_len);
        //         // for c in tag_name.0.as_bytes() {
        //         //     bytes.push(*c);
        //         // }
        //         bytes.append(&mut tag_name.bytes());
        //     }
        // }
        // println!("mani hash len: {}", bytes.len());
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    //TODO: There can be up to 256 Tags defined in a manifest,
    // each 32 bytes long, so a Manifest will be at most 9 Data chunks long
    // (later there can be some additions).
    // There is only one exception to this, when there are no Tags defined,
    // Then it can be only 1 Data long
    // If we add a single Tag, then Manifest has to expand to 2 Datas.
    // Tags are ordered from 0 to 255.
    // Each Tag is stored as a sequence of 32 bytes in Data, if all of these bytes are 0,
    // then given Tag is not defined, otherwise given Tag is defined.
    pub fn from(data_vec: Vec<Data>) -> Self {
        let data_count = data_vec.len();
        if data_count == 0 {
            return Manifest {
                app_type: AppType::Other(0),
                description: String::new(),
                tags: HashMap::new(),
                d_types: HashMap::new(),
            };
        }
        eprintln!("Constructing manifest from: {} Data blocks", data_count);
        let mut data_iter = data_vec.into_iter();
        let first_data = data_iter.next().unwrap();
        let mut iter = first_data.bytes().into_iter();
        if data_count == 1 && iter.len() == 1 {
            return Manifest {
                app_type: AppType::from(iter.next().unwrap()),
                description: String::new(),
                tags: HashMap::new(),
                d_types: HashMap::new(),
            };
        }
        let app_type = if let Some(byte) = iter.next() {
            AppType::from(byte)
        } else {
            AppType::Other(0)
        };
        // let tags_count = if let Some(byte) = iter.next() {
        //     byte as usize
        // } else {
        //     0
        // };
        let first_tags_page = u16::from_be_bytes([iter.next().unwrap(), iter.next().unwrap()]);
        let first_dt_page = u16::from_be_bytes([iter.next().unwrap(), iter.next().unwrap()]);
        let other_defs = u16::from_be_bytes([iter.next().unwrap(), iter.next().unwrap()]);
        let tag_pages_count = if first_tags_page == 0 {
            0
        } else {
            if first_dt_page == 0 && other_defs == 0 {
                data_count - 1
            } else if first_dt_page > 0 {
                first_dt_page as usize - first_tags_page as usize
            } else {
                other_defs as usize - first_tags_page as usize
            }
        };
        let mut tags = HashMap::with_capacity(tag_pages_count << 5);
        // let d_types_count = if let Some(byte) = iter.next() {
        //     byte as usize
        // } else {
        //     0
        // };
        let dt_page_count = if other_defs == 0 {
            data_count - first_dt_page as usize
        } else {
            other_defs as usize - first_dt_page as usize
        };

        let mut d_types = HashMap::with_capacity(dt_page_count << 5);
        // let _tags_len = iter.next().unwrap();
        let description = String::from_utf8(iter.collect()).unwrap();

        let mut current_tag_id: u8 = 0;
        let mut tag_pages_read = 0;
        // let mut current_dtype_id = 0;
        let mut dtype_pages_read = 0;
        let mut adding_tags = true;
        while let Some(data) = data_iter.next() {
            let bytes = data.bytes();
            for chunk in bytes.chunks_exact(32) {
                let mut all_zeros = true;
                let mut non_space_byte_occured = false;
                let mut name_bytes = Vec::with_capacity(32);
                for byte in chunk {
                    if *byte > 0 {
                        all_zeros = false;
                    }
                    if *byte == 32 {
                        if non_space_byte_occured {
                            name_bytes.push(*byte);
                        }
                    } else {
                        non_space_byte_occured = true;
                        name_bytes.push(*byte);
                    }
                }
                if !all_zeros {
                    let tag = Tag::new(String::from_utf8(name_bytes).unwrap()).unwrap();
                    if adding_tags {
                        tags.insert(current_tag_id, tag);
                    } else {
                        d_types.insert(current_tag_id, tag);
                    }
                    // tags_added = tags_added + 1;
                }
                current_tag_id = current_tag_id.saturating_add(1);
                // if adding_tags && tags_added >= tags_count {
                //     adding_tags = false;
                //     tags_added = 0;
                //     current_tag_id = 0;
                // }
            }
            if adding_tags {
                tag_pages_read = tag_pages_read + 1;
                if tag_pages_read >= tag_pages_count {
                    adding_tags = false;
                    current_tag_id = 0;
                }
            } else {
                dtype_pages_read = dtype_pages_read + 1;
                if dtype_pages_read >= dt_page_count {
                    // We don't want to tread other data as data type defs
                    break;
                }
            }
        }
        // TODO: read other data if any!
        Self {
            app_type,
            description,
            tags,
            d_types,
        }
    }

    pub fn to_data(&self) -> Vec<Data> {
        let mut res = Vec::with_capacity(1024);
        res.push(self.app_type.byte());
        let tags_len = self.tags.len() as u8;
        res.push(0);
        if tags_len == 0 {
            res.push(0);
        } else {
            res.push(1);
        }
        let d_types_len = self.d_types.len() as u8;
        res.push(0);
        if d_types_len == 0 {
            res.push(0);
        } else {
            //TODO: Calculate d_id of first page containing user defined data types
            // it is right after Tags pages
            let tags_pages_count = tags_len >> 4 + if tags_len % 32 > 0 { 1 } else { 0 };
            res.push(tags_pages_count + 1);
        }
        // TODO: index of other data after Data type definitions, 0 if none
        res.push(0);
        res.push(0);
        for byte in self.description.bytes() {
            res.push(byte);
        }
        let first_data_bytes = std::mem::replace(&mut res, Vec::with_capacity(1024));
        let mut output = vec![Data::new(first_data_bytes).unwrap()];
        let mut tags_to_add = self.tags.len();
        let mut elements_pushed = 0;
        for i in 0..=255 {
            if let Some(tag) = self.tags.get(&i) {
                res.append(&mut tag.bytes());
                tags_to_add -= 1;
            } else {
                res.append(&mut vec![0; 32]);
            }
            elements_pushed += 1;
            if tags_to_add == 0 {
                break;
            }
            if elements_pushed >= 32 {
                elements_pushed = 0;
                let next_data_bytes = std::mem::replace(&mut res, Vec::with_capacity(1024));
                output.push(Data::new(next_data_bytes).unwrap());
            }
        }
        if res.len() > 0 {
            let next_data_bytes = std::mem::replace(&mut res, Vec::with_capacity(1024));
            output.push(Data::new(next_data_bytes).unwrap());
        }
        let mut d_types_to_add = self.d_types.len();
        let mut elements_pushed = 0;
        for i in 0..=255 {
            if let Some(tag) = self.d_types.get(&i) {
                res.append(&mut tag.bytes());
                d_types_to_add -= 1;
            } else {
                res.append(&mut vec![0; 32]);
            }
            elements_pushed += 1;
            if d_types_to_add == 0 {
                break;
            }
            if elements_pushed >= 32 {
                elements_pushed = 0;
                let next_data_bytes = std::mem::replace(&mut res, Vec::with_capacity(1024));
                output.push(Data::new(next_data_bytes).unwrap());
            }
        }
        if res.len() > 0 {
            let next_data_bytes = std::mem::replace(&mut res, Vec::with_capacity(1024));
            output.push(Data::new(next_data_bytes).unwrap());
        }

        output
    }

    pub fn add_tags(&mut self, tags: Vec<Tag>) -> bool {
        eprintln!("add_tags {:?}", tags);
        let mut any_tag_added = false;
        let mut existing_tags = Vec::with_capacity(self.tags.len());
        for (_id, tag) in &self.tags {
            // eprintln!("add_tags existing:{} {}", _id, tag.0);
            existing_tags.push(tag.clone());
        }
        let mut tags_to_add = vec![];
        let tags_iter = tags.into_iter();
        for tag in tags_iter {
            if !existing_tags.contains(&tag) {
                // eprintln!("add_tags new: {}", tag.0);
                tags_to_add.push(tag);
            }
        }
        let mut tags_iter = tags_to_add.into_iter();
        let mut last_id_checked = 0;
        while let Some(tag) = tags_iter.next() {
            for i in last_id_checked..=255 {
                if self.tags.contains_key(&i) {
                    continue;
                }
                any_tag_added = true;
                // eprintln!("add_tags insert:{} {}", i, tag.0);
                self.tags.insert(i, tag);
                last_id_checked = i;
                break;
            }
        }
        any_tag_added
    }

    pub fn add_data_type(&mut self, tag: Tag) -> bool {
        eprintln!("Adding dtype {} to manifest", tag.0);
        let mut added = false;
        if self.d_types.len() >= 256 {
            return added;
        }
        for i in 0..=255 {
            if !self.d_types.contains_key(&i) {
                self.d_types.insert(i, tag);
                added = true;
                break;
            }
        }
        added
    }

    // TODO: before deleting a Tag make sure there is no Content labeled with it
    // If there is any, first modify all Content's headers to drop that label
    // on only then call this fn
    pub fn del_tags(&mut self, tags: Vec<Tag>) {
        for id in 0..=255 {
            if let Some(tag) = self.tags.get(&id) {
                if tags.contains(tag) {
                    self.tags.remove(&id);
                }
            }
        }
    }
    pub fn rename_tags(&mut self, mut tags: HashMap<Tag, Tag>) {
        for id in 0..=255 {
            if let Some(tag) = self.tags.get(&id) {
                if tags.contains_key(tag) {
                    let new_tag = tags.remove(&tag).unwrap();
                    self.tags.insert(id, new_tag);
                }
            }
            if tags.is_empty() {
                break;
            }
        }
    }
    pub fn tag_names(&self) -> Vec<String> {
        let mut tag_names = Vec::with_capacity(256);
        for id in 0..=255 {
            if let Some(tag) = self.tags.get(&id) {
                tag_names.push(tag.0.clone());
            }
        }
        tag_names
    }
    pub fn dtype_names(&self) -> Vec<String> {
        let mut type_names = Vec::with_capacity(256);
        for id in 0..=255 {
            if let Some(tag) = self.d_types.get(&id) {
                type_names.push(tag.0.clone());
            }
        }
        type_names
    }
}
