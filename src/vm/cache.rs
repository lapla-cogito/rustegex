use foldhash::HashMapExt as _;

const BITMAP_BIT_LIMIT: usize = 256;
const MAX_BITMAP_BYTES: usize = 16 * 1024 * 1024; // 16 MB
const CACHE_CAPACITY: usize = 4096;

#[derive(Debug)]
pub enum Cache {
    Bitmap(BitmapCache),
    Fallback(FallbackCache),
}

#[derive(Debug)]
pub struct BitmapCache {
    bitmap: Vec<u64>,
    stride: usize,
    input_size: usize,
}

#[derive(Debug)]
pub struct FallbackCache {
    map: foldhash::HashMap<(usize, usize), ()>,
    queue: std::collections::VecDeque<(usize, usize)>,
    capacity: usize,
}

impl Cache {
    pub fn new(program_size: usize, input_size: usize) -> Self {
        let stride = program_size.div_ceil(64);
        let bitmap_bytes = input_size.saturating_mul(stride).saturating_mul(8);

        if program_size <= BITMAP_BIT_LIMIT && bitmap_bytes <= MAX_BITMAP_BYTES {
            Cache::Bitmap(BitmapCache::new(program_size, input_size))
        } else {
            Cache::Fallback(FallbackCache::new(CACHE_CAPACITY))
        }
    }

    #[inline]
    pub fn contains(&self, input_pos: usize, pc: usize) -> bool {
        match self {
            Cache::Bitmap(cache) => cache.contains(input_pos, pc),
            Cache::Fallback(cache) => cache.contains(input_pos, pc),
        }
    }

    #[inline]
    pub fn insert(&mut self, input_pos: usize, pc: usize) {
        match self {
            Cache::Bitmap(cache) => cache.insert(input_pos, pc),
            Cache::Fallback(cache) => cache.insert(input_pos, pc),
        }
    }
}

impl BitmapCache {
    fn new(program_size: usize, input_size: usize) -> Self {
        let stride = program_size.div_ceil(64);
        let len = (input_size + 1) * stride;
        let bitmap = vec![0u64; len];

        BitmapCache {
            bitmap,
            stride,
            input_size,
        }
    }

    #[inline]
    fn contains(&self, input_pos: usize, pc: usize) -> bool {
        if input_pos > self.input_size {
            return false;
        }

        let word_offset = pc / 64;
        let bit_idx = pc % 64;

        if word_offset >= self.stride {
            return false;
        }

        let index = input_pos * self.stride + word_offset;
        // Safety: index calculation is bounded by input_size * stride + word_offset
        // which is < (input_size + 1) * stride = len
        unsafe { (*self.bitmap.get_unchecked(index) & (1u64 << bit_idx)) != 0 }
    }

    #[inline]
    fn insert(&mut self, input_pos: usize, pc: usize) {
        if input_pos > self.input_size {
            return;
        }

        let word_offset = pc / 64;
        let bit_idx = pc % 64;

        if word_offset >= self.stride {
            return;
        }

        let index = input_pos * self.stride + word_offset;
        unsafe {
            *self.bitmap.get_unchecked_mut(index) |= 1u64 << bit_idx;
        }
    }

    fn clear(&mut self) {
        self.bitmap.fill(0);
    }
}

impl FallbackCache {
    fn new(capacity: usize) -> Self {
        FallbackCache {
            map: foldhash::HashMap::with_capacity(capacity),
            queue: std::collections::VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    #[inline]
    fn contains(&self, input_pos: usize, pc: usize) -> bool {
        self.map.contains_key(&(input_pos, pc))
    }

    #[inline]
    fn insert(&mut self, input_pos: usize, pc: usize) {
        let key = (input_pos, pc);
        if self.map.contains_key(&key) {
            return;
        }

        if self.queue.len() >= self.capacity {
            if let Some(oldest) = self.queue.pop_front() {
                self.map.remove(&oldest);
            }
        }

        self.queue.push_back(key);
        self.map.insert(key, ());
    }

    fn clear(&mut self) {
        self.map.clear();
        self.queue.clear();
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

        let mut cache = if let Some(mut existing_cache) = cache_opt.take() {
            let stride = program_size.div_ceil(64);
            let bitmap_bytes = input_size.saturating_mul(stride).saturating_mul(8);
            let use_bitmap = program_size <= BITMAP_BIT_LIMIT && bitmap_bytes <= MAX_BITMAP_BYTES;

            match existing_cache {
                Cache::Bitmap(ref mut b) => {
                    if use_bitmap {
                        let needed_len = (input_size + 1) * stride;
                        if b.stride == stride && b.bitmap.len() >= needed_len {
                            // If existing is huge (>1MB) and needed is tiny (<4KB), discard to save memory/clear time
                            if b.bitmap.len() > 1024 * 1024 && needed_len < 4096 {
                                Cache::new(program_size, input_size)
                            } else {
                                b.input_size = input_size;
                                b.clear();
                                existing_cache
                            }
                        } else {
                            Cache::new(program_size, input_size)
                        }
                    } else {
                        Cache::new(program_size, input_size)
                    }
                }
                Cache::Fallback(ref mut f) => {
                    if !use_bitmap {
                        f.clear();
                        existing_cache
                    } else {
                        Cache::new(program_size, input_size)
                    }
                }
            }
        } else {
            Cache::new(program_size, input_size)
        };

        let result = f(&mut cache);

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
    fn test_fallback_cache() {
        let mut cache = FallbackCache::new(3);

        assert!(!cache.contains(0, 0));

        cache.insert(0, 0);
        cache.insert(1, 1);
        cache.insert(2, 2);

        assert!(cache.contains(0, 0));
        assert!(cache.contains(1, 1));
        assert!(cache.contains(2, 2));
        assert_eq!(cache.queue.len(), 3);

        cache.insert(3, 3);
        assert_eq!(cache.queue.len(), 3);
        assert!(!cache.contains(0, 0));
        assert!(cache.contains(3, 3));

        cache.clear();
        assert!(!cache.contains(1, 1));
        assert!(!cache.contains(2, 2));
        assert!(!cache.contains(3, 3));
        assert_eq!(cache.queue.len(), 0);
    }

    #[test]
    fn test_cache_selection() {
        let cache = Cache::new(10, 10);
        assert!(matches!(cache, Cache::Bitmap(_)));

        let cache = Cache::new(10, 1_000_000);
        assert!(matches!(cache, Cache::Bitmap(_)));

        let cache = Cache::new(300, 10);
        assert!(matches!(cache, Cache::Fallback(_)));

        let cache = Cache::new(10, 10_000_000);
        assert!(matches!(cache, Cache::Fallback(_)));
    }
}
