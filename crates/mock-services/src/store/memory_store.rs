//! 内存存储
//!
//! 使用 DashMap 实现的高并发内存存储，适用于测试和开发环境。

use dashmap::DashMap;
use std::sync::Arc;

/// 通用内存存储
///
/// 基于 DashMap 实现，支持高并发读写操作。
/// 适用于存储模拟服务的临时数据，如用户、订单、优惠券等。
#[derive(Debug)]
pub struct MemoryStore<T> {
    data: Arc<DashMap<String, T>>,
}

impl<T: Clone> Default for MemoryStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> MemoryStore<T> {
    /// 创建新的内存存储实例
    pub fn new() -> Self {
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// 插入或更新数据
    ///
    /// 如果 key 已存在则覆盖原有数据
    pub fn insert(&self, id: &str, value: T) {
        self.data.insert(id.to_string(), value);
    }

    /// 获取数据
    ///
    /// 返回数据的克隆，不持有锁
    pub fn get(&self, id: &str) -> Option<T> {
        self.data.get(id).map(|v| v.clone())
    }

    /// 删除数据
    ///
    /// 返回被删除的数据
    pub fn remove(&self, id: &str) -> Option<T> {
        self.data.remove(id).map(|(_, v)| v)
    }

    /// 列出所有数据
    ///
    /// 返回所有值的克隆列表
    pub fn list(&self) -> Vec<T> {
        self.data
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 按条件筛选数据
    ///
    /// 返回满足条件的所有数据
    pub fn list_by<F>(&self, predicate: F) -> Vec<T>
    where
        F: Fn(&T) -> bool,
    {
        self.data
            .iter()
            .filter(|entry| predicate(entry.value()))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 获取数据总数
    pub fn count(&self) -> usize {
        self.data.len()
    }

    /// 清空所有数据
    pub fn clear(&self) {
        self.data.clear();
    }

    /// 检查是否存在指定 key
    pub fn contains(&self, id: &str) -> bool {
        self.data.contains_key(id)
    }

    /// 批量插入数据
    ///
    /// 接收一个迭代器，提取每个元素的 key 并插入
    pub fn insert_many<I, F>(&self, items: I, key_fn: F)
    where
        I: IntoIterator<Item = T>,
        F: Fn(&T) -> String,
    {
        for item in items {
            let key = key_fn(&item);
            self.data.insert(key, item);
        }
    }
}

impl<T: Clone> Clone for MemoryStore<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestItem {
        id: String,
        value: i32,
    }

    #[test]
    fn test_memory_store_crud() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        // Create
        let item = TestItem {
            id: "test-1".to_string(),
            value: 42,
        };
        store.insert("test-1", item.clone());

        // Read
        let retrieved = store.get("test-1").unwrap();
        assert_eq!(retrieved, item);

        // Update
        let updated_item = TestItem {
            id: "test-1".to_string(),
            value: 100,
        };
        store.insert("test-1", updated_item.clone());
        let retrieved = store.get("test-1").unwrap();
        assert_eq!(retrieved.value, 100);

        // Delete
        let removed = store.remove("test-1").unwrap();
        assert_eq!(removed.value, 100);
        assert!(store.get("test-1").is_none());
    }

    #[test]
    fn test_memory_store_list() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        store.insert(
            "1",
            TestItem {
                id: "1".to_string(),
                value: 10,
            },
        );
        store.insert(
            "2",
            TestItem {
                id: "2".to_string(),
                value: 20,
            },
        );
        store.insert(
            "3",
            TestItem {
                id: "3".to_string(),
                value: 30,
            },
        );

        let all = store.list();
        assert_eq!(all.len(), 3);
        assert_eq!(store.count(), 3);
    }

    #[test]
    fn test_memory_store_list_by() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        store.insert(
            "1",
            TestItem {
                id: "1".to_string(),
                value: 10,
            },
        );
        store.insert(
            "2",
            TestItem {
                id: "2".to_string(),
                value: 20,
            },
        );
        store.insert(
            "3",
            TestItem {
                id: "3".to_string(),
                value: 30,
            },
        );

        // 筛选 value > 15 的项
        let filtered = store.list_by(|item| item.value > 15);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|item| item.value > 15));
    }

    #[test]
    fn test_memory_store_clear() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        store.insert(
            "1",
            TestItem {
                id: "1".to_string(),
                value: 10,
            },
        );
        store.insert(
            "2",
            TestItem {
                id: "2".to_string(),
                value: 20,
            },
        );

        assert_eq!(store.count(), 2);
        store.clear();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_memory_store_contains() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        store.insert(
            "exists",
            TestItem {
                id: "exists".to_string(),
                value: 1,
            },
        );

        assert!(store.contains("exists"));
        assert!(!store.contains("not-exists"));
    }

    #[test]
    fn test_memory_store_insert_many() {
        let store: MemoryStore<TestItem> = MemoryStore::new();

        let items = vec![
            TestItem {
                id: "a".to_string(),
                value: 1,
            },
            TestItem {
                id: "b".to_string(),
                value: 2,
            },
            TestItem {
                id: "c".to_string(),
                value: 3,
            },
        ];

        store.insert_many(items, |item| item.id.clone());

        assert_eq!(store.count(), 3);
        assert!(store.contains("a"));
        assert!(store.contains("b"));
        assert!(store.contains("c"));
    }
}
