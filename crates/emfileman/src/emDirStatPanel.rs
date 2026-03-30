use crate::emDirEntry::emDirEntry;

pub struct DirStatistics {
    pub total_count: i32,
    pub file_count: i32,
    pub sub_dir_count: i32,
    pub other_type_count: i32,
    pub hidden_count: i32,
}

impl DirStatistics {
    pub fn from_entries(entries: &[emDirEntry]) -> Self {
        let mut s = Self {
            total_count: 0,
            file_count: 0,
            sub_dir_count: 0,
            other_type_count: 0,
            hidden_count: 0,
        };
        for e in entries {
            s.total_count += 1;
            if e.IsHidden() {
                s.hidden_count += 1;
            }
            if e.IsDirectory() {
                s.sub_dir_count += 1;
            } else if e.IsRegularFile() {
                s.file_count += 1;
            } else {
                s.other_type_count += 1;
            }
        }
        s
    }

    pub fn format_text(&self) -> String {
        format!(
            "Directory Statistics\n\
             ~~~~~~~~~~~~~~~~~~~~\n\
             \n\
             Total Entries : {:5}\n\
             Hidden Entries: {:5}\n\
             Regular Files : {:5}\n\
             Subdirectories: {:5}\n\
             Other Types   : {:5}",
            self.total_count,
            self.hidden_count,
            self.file_count,
            self.sub_dir_count,
            self.other_type_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emDirEntry::emDirEntry;

    #[test]
    fn count_entries() {
        let entries = vec![
            emDirEntry::from_path("/tmp"),
            emDirEntry::from_path("/dev/null"),
        ];
        let stats = DirStatistics::from_entries(&entries);
        assert_eq!(stats.total_count, 2);
        assert!(stats.sub_dir_count >= 1); // /tmp is a directory
    }

    #[test]
    fn empty_entries() {
        let stats = DirStatistics::from_entries(&[]);
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.sub_dir_count, 0);
        assert_eq!(stats.other_type_count, 0);
        assert_eq!(stats.hidden_count, 0);
    }

    #[test]
    fn hidden_count() {
        let dir = std::env::temp_dir();
        let hidden = dir.join(".test_hidden_stat_emfileman");
        std::fs::write(&hidden, "x").unwrap();
        let entries = vec![emDirEntry::from_path(hidden.to_str().unwrap())];
        let stats = DirStatistics::from_entries(&entries);
        assert_eq!(stats.hidden_count, 1);
        std::fs::remove_file(&hidden).unwrap();
    }

    #[test]
    fn format_text_output() {
        let stats = DirStatistics {
            total_count: 10,
            file_count: 5,
            sub_dir_count: 3,
            other_type_count: 2,
            hidden_count: 1,
        };
        let text = stats.format_text();
        assert!(text.contains("Total Entries :    10"));
        assert!(text.contains("Hidden Entries:     1"));
    }
}
