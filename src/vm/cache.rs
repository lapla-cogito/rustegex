use foldhash::HashMapExt as _;

const BITMAP_THRESHOLD: usize = 64;
const LRU_CAPACITY: usize = 1024;

#[derive(Debug)]
pub enum Cache {
    Bitmap(BitmapCache),
    Lru(LruCache),
}

#[derive(Debug)]
pub struct BitmapCache {
    bitmap: Vec<Vec<u64>>,
    pc_words: usize,
}

#[derive(Debug)]
pub struct LruCache {
    cache: foldhash::HashMap<(usize, usize), usize>, // (input_pos, pc) -> access_order
    capacity: usize,
    access_counter: usize,
}

impl Cache {
    pub fn new(program_size: usize, input_size: usize) -> Self {
        if program_size < BITMAP_THRESHOLD && input_size < BITMAP_THRESHOLD {
            Cache::Bitmap(BitmapCache::new(program_size, input_size))
        } else {
            Cache::Lru(LruCache::new(LRU_CAPACITY))
        }
    }

    pub fn contains(&self, input_pos: usize, pc: usize) -> bool {
        match self {
            Cache::Bitmap(cache) => cache.contains(input_pos, pc),
            Cache::Lru(cache) => cache.contains(input_pos, pc),
        }
    }

    pub fn insert(&mut self, input_pos: usize, pc: usize) {
        match self {
            Cache::Bitmap(cache) => cache.insert(input_pos, pc),
            Cache::Lru(cache) => cache.insert(input_pos, pc),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Cache::Bitmap(cache) => cache.clear(),
            Cache::Lru(cache) => cache.clear(),
        }
    }
}

impl BitmapCache {
    fn new(program_size: usize, input_size: usize) -> Self {
        let pc_words = program_size.div_ceil(64);
        let bitmap = vec![vec![0u64; pc_words]; input_size + 1];

        BitmapCache { bitmap, pc_words }
    }

    fn contains(&self, input_pos: usize, pc: usize) -> bool {
        if input_pos >= self.bitmap.len() {
            return false;
        }

        let word_idx = pc / 64;
        let bit_idx = pc % 64;

        if word_idx >= self.pc_words {
            return false;
        }

        (self.bitmap[input_pos][word_idx] & (1u64 << bit_idx)) != 0
    }

    fn insert(&mut self, input_pos: usize, pc: usize) {
        if input_pos >= self.bitmap.len() {
            return;
        }

        let word_idx = pc / 64;
        let bit_idx = pc % 64;

        if word_idx >= self.pc_words {
            return;
        }

        self.bitmap[input_pos][word_idx] |= 1u64 << bit_idx;
    }

    fn clear(&mut self) {
        for row in &mut self.bitmap {
            for word in row {
                *word = 0;
            }
        }
    }
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        LruCache {
            cache: foldhash::HashMap::with_capacity(capacity),
            capacity,
            access_counter: 0,
        }
    }

    fn contains(&self, input_pos: usize, pc: usize) -> bool {
        self.cache.contains_key(&(input_pos, pc))
    }

    fn insert(&mut self, input_pos: usize, pc: usize) {
        let key = (input_pos, pc);

        if self.cache.len() >= self.capacity
            && !self.cache.contains_key(&key)
            && let Some(oldest_key) = self.find_oldest_key()
        {
            self.cache.remove(&oldest_key);
        }

        self.access_counter += 1;
        self.cache.insert(key, self.access_counter);
    }

    fn find_oldest_key(&self) -> Option<(usize, usize)> {
        self.cache
            .iter()
            .min_by_key(|&(_, access_time)| access_time)
            .map(|(key, _)| *key)
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.access_counter = 0;
    }
}

thread_local! {
    static THREAD_CACHE: std::cell::RefCell<Option<Cache>> = const { std::cell::RefCell::new(None) };
}

pub fn with_thread_cache<F, R>(program_size: usize, input_size: usize, f: F) -> R
where
    F: FnOnce(&mut Cache) -> R,
{
    THREAD_CACHE.with(|cache_cell| {
        let mut cache_opt = cache_cell.borrow_mut();

        let mut cache = match cache_opt.take() {
            Some(mut existing_cache) => {
                existing_cache.clear();

                let should_use_bitmap =
                    program_size < BITMAP_THRESHOLD && input_size < BITMAP_THRESHOLD;
                let is_bitmap = matches!(existing_cache, Cache::Bitmap(_));

                if should_use_bitmap == is_bitmap {
                    existing_cache
                } else {
                    Cache::new(program_size, input_size)
                }
            }
            None => Cache::new(program_size, input_size),
        };

        let result = f(&mut cache);

        // recover cache
        *cache_opt = Some(cache);
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_cache() {
        let mut cache = BitmapCache::new(10, 5);

        assert!(!cache.contains(0, 0));
        assert!(!cache.contains(3, 7));

        cache.insert(0, 0);
        cache.insert(3, 7);
        cache.insert(2, 9);

        assert!(cache.contains(0, 0));
        assert!(cache.contains(3, 7));
        assert!(cache.contains(2, 9));

        assert!(!cache.contains(1, 0));
        assert!(!cache.contains(0, 1));

        cache.clear();
        assert!(!cache.contains(0, 0));
        assert!(!cache.contains(3, 7));
    }

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(3);

        assert!(!cache.contains(0, 0));

        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);

        assert!(cache.contains(0, 0));
        assert!(cache.contains(1, 1));
        assert!(cache.contains(2, 2));
        assert_eq!(cache.cache.len(), 3);

        cache.insert(3, 3);
        assert_eq!(cache.cache.len(), 3);
        assert!(!cache.contains(0, 0));
        assert!(cache.contains(3, 3));

        cache.clear();
        assert!(!cache.contains(1, 1));
        assert!(!cache.contains(2, 2));
        assert!(!cache.contains(3, 3));
        assert_eq!(cache.cache.len(), 0);
    }

    #[test]
    fn test_cache_selection() {
        let cache = Cache::new(10, 10);
        assert!(matches!(cache, Cache::Bitmap(_)));

        let cache = Cache::new(100, 10);
        assert!(matches!(cache, Cache::Lru(_)));

        let cache = Cache::new(10, 100);
        assert!(matches!(cache, Cache::Lru(_)));
    }

    #[test]
    fn test_thread_cache() {
        let result = with_thread_cache(10, 10, |cache| {
            assert!(!cache.contains(0, 0));
            cache.insert(0, 0);
            assert!(cache.contains(0, 0));
            true
        });

        assert!(result);

        with_thread_cache(10, 10, |cache| {
            assert!(!cache.contains(0, 0));
        });
    }
}
