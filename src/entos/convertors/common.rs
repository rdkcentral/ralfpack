//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2025 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use crate::entos::config_xml;
use std::collections::HashSet;

/// Helper function to split a string using a comma as a delimiter and return a set of unique
/// values.  This will trim each part of the string to remove any leading or trailing whitespace.
/// It will also ignore empty strings.
pub fn split_to_set(s: &str, sep: char) -> HashSet<String> {
    let items: HashSet<String> = s
        .split(sep)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    items
}

/// Helper function that converts a set of strings into a single string with each value
/// separated by the specified separator character.
pub fn set_to_string(set: &HashSet<String>, sep: char) -> String {
    if set.is_empty() {
        String::new()
    } else {
        let mut items: Vec<&str> = set.iter().map(|s| s.trim()).collect();
        items.sort();
        items.join(&sep.to_string())
    }
}

/// Helper function to split a string using a comma as a delimiter and return a vector of values.
/// This will trim each part of the string to remove any leading or trailing whitespace.
/// It will also ignore empty strings.
pub fn split_to_vec(s: &str, sep: char) -> Vec<String> {
    let items: Vec<String> = s
        .split(sep)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    items
}

/// Helper function that converts a set of strings into a single string with each value
/// separated by the specified separator character.
pub fn vec_to_string(vec: &Vec<String>, sep: char) -> String {
    if vec.is_empty() {
        String::new()
    } else {
        let items: Vec<&str> = vec.iter().map(|s| s.trim()).collect();
        items.join(&sep.to_string())
    }
}

pub trait FromCapabilities
where
    Self: Sized,
{
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self>;
}

pub trait ToCapabilities {
    fn to_capabilities(&self) -> Vec<config_xml::Capability>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_to_set() {
        let s = "one, two, three , four , five";
        let set = split_to_set(s, ',');
        assert_eq!(set.len(), 5);
        assert!(set.contains("one"));
        assert!(set.contains("two"));
        assert!(set.contains("three"));
        assert!(set.contains("four"));
        assert!(set.contains("five"));

        let s = "  one  ,  two  ,  three  ";
        let set = split_to_set(s, ',');
        assert_eq!(set.len(), 3);
        assert!(set.contains("one"));
        assert!(set.contains("two"));
        assert!(set.contains("three"));

        let s = "one,,two,,three,,";
        let set = split_to_set(s, ',');
        assert_eq!(set.len(), 3);
        assert!(set.contains("one"));
        assert!(set.contains("two"));
        assert!(set.contains("three"));

        let s = ", , , ,";
        let set = split_to_set(s, ',');
        assert!(set.is_empty());

        let s = "";
        let set = split_to_set(s, ',');
        assert!(set.is_empty());
    }

    #[test]
    fn test_split_to_vec() {
        let s = "one, two, three , four , five";
        let vec = split_to_vec(s, ',');
        assert_eq!(vec.len(), 5);
        assert_eq!(vec[0], "one");
        assert_eq!(vec[1], "two");
        assert_eq!(vec[2], "three");
        assert_eq!(vec[3], "four");
        assert_eq!(vec[4], "five");

        let s = "  one  ,  two  ,  three  ";
        let vec = split_to_vec(s, ',');
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], "one");
        assert_eq!(vec[1], "two");
        assert_eq!(vec[2], "three");

        let s = "one;;two ; three;;";
        let vec = split_to_vec(s, ';');
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], "one");
        assert_eq!(vec[1], "two");
        assert_eq!(vec[2], "three");

        let s = ", , , ,";
        let vec = split_to_vec(s, ',');
        assert!(vec.is_empty());

        let s = "";
        let vec = split_to_vec(s, ',');
        assert!(vec.is_empty());
    }

    #[test]
    fn test_set_to_string() {
        let set = HashSet::new();
        let s = set_to_string(&set, ',');
        assert!(s.is_empty());
        assert_eq!(s, "");

        let set = HashSet::from(["aaa".to_string(), " bbb  ".to_string(), "ccc   ".to_string()]);
        let s = set_to_string(&set, ';');
        assert!(!s.is_empty());
        assert_eq!(s, "aaa;bbb;ccc");
    }

    #[test]
    fn test_vec_to_string() {
        let v = Vec::new();
        let s = vec_to_string(&v, ',');
        assert!(s.is_empty());
        assert_eq!(s, "");

        let v = Vec::from(["one".to_string(), "two".to_string(), "three".to_string()]);
        let s = vec_to_string(&v, ',');
        assert!(!s.is_empty());
        assert_eq!(s, "one,two,three");

        let v = Vec::from(["one  ".to_string(), "  two".to_string(), "three".to_string()]);
        let s = vec_to_string(&v, ',');
        assert!(!s.is_empty());
        assert_eq!(s, "one,two,three");
    }
}
