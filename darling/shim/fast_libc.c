/* Fast memory and string functions.
 *
 * Darling's libsystem_platform, which provides memcpy/memmove, memset,
 * bzero, memcmp, memchr, strlen, strcmp, strncmp and memset_pattern4/8/16
 * for every Mach-O program, is built without optimization: each loop step
 * stores and reloads its variables through the stack, memmove copies at
 * most a word at a time, and memset_pattern16 calls that memmove for every
 * 16 bytes. Apple ships hand-written assembly there. Roblox copies memory
 * constantly; in its menu a third of the main thread went to
 * _platform_memmove. These replacements are interposed for all images.
 *
 * Built with -O2 on its own (see build_debug_shim.sh) and without builtins,
 * so the compiler cannot turn these loops back into calls to themselves.
 *
 * `rep movsb` and `rep stosb` are used only where they beat a plain SSE loop
 * (measured on a Ryzen 7 5800X, Zen 3):
 *  - they need ~35 cycles to start, so short copies and fills use SSE loops
 *    (a 128-byte copy took 7 ns instead of 2);
 *  - past the 512 KB L2 cache they fall far behind (a 1 MB copy took 4 times
 *    as long as the loop, a 1 MB fill nearly 3 times), so large ones use
 *    loops too;
 *  - `rep movsb` crawls when the destination is a little above the source
 *    modulo 4 KB: its loads wait for earlier stores to the same page offset
 *    ("4K aliasing"). With the destination 1 to 31 bytes above, an 8 KB copy
 *    took 2.2 us instead of 60 ns, and it stays slower up to ~600 bytes
 *    (except at multiples of 32). A forward loop has the same problem, a
 *    backward one does not, so such copies run backwards when the buffers
 *    do not overlap. */
typedef unsigned long size_t;
typedef unsigned long u64;
typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
typedef char v16 __attribute__((vector_size(16), aligned(1), may_alias)); /* any address */
typedef char v16a __attribute__((vector_size(16), may_alias));             /* 16-byte aligned */
typedef u8 u8x16 __attribute__((vector_size(16)));
typedef u32 u32x4 __attribute__((vector_size(16)));
typedef u64 u64x2 __attribute__((vector_size(16)));
typedef u64 u64u __attribute__((aligned(1), may_alias));
typedef u32 u32u __attribute__((aligned(1), may_alias));
typedef u16 u16u __attribute__((aligned(1), may_alias));

extern void *memcpy(void *, const void *, size_t);
extern void *memmove(void *, const void *, size_t);
extern void *memset(void *, int, size_t);
extern void bzero(void *, size_t);
extern int memcmp(const void *, const void *, size_t);
extern void *memchr(const void *, int, size_t);
extern size_t strlen(const char *);
extern int strcmp(const char *, const char *);
extern int strncmp(const char *, const char *, size_t);
extern void memset_pattern4(void *, const void *, size_t);
extern void memset_pattern8(void *, const void *, size_t);
extern void memset_pattern16(void *, const void *, size_t);

#define DYLD_INTERPOSE(_replacement, _replacee) \
    __attribute__((used)) static struct { const void *replacement; const void *replacee; } \
    _interpose_##_replacee __attribute__((section("__DATA,__interpose"))) = \
        {(const void *)(unsigned long)&_replacement, (const void *)(unsigned long)&_replacee};

#define INLINE static inline __attribute__((always_inline))
#define LOAD(p) (*(const v16 *)(p))
#define STORE(p, v) (*(v16 *)(p) = (v))
#define STORE_ALIGNED(p, v) (*(v16a *)(p) = (v16a)(v))
#define REP_MOVSB_MIN 2048
#define REP_STOSB_MIN 4096
#define REP_MOVSB_MAX (256UL << 10)
#define REP_STOSB_MAX (512UL << 10)
#define ALIAS_WINDOW 640 /* destination this far above the source (mod 4 KB) */

/* Bit i set where byte i of `a` equals byte i of `b`. */
INLINE u32 equal_mask(v16 a, v16 b) { return (u32)__builtin_ia32_pmovmskb128((v16)(a == b)); }
INLINE u32 lowest(u32 mask) { return (u32)__builtin_ctz(mask); }
/* Unsigned byte minimum (pminub); __builtin_elementwise_min needs clang 14. */
INLINE u8x16 min_u8(u8x16 a, u8x16 b) { return b ^ ((a ^ b) & (u8x16)(a < b)); }

/* Up to 64 bytes; every load happens before any store, so overlap is fine. */
INLINE void copy_small(u8 *d, const u8 *s, size_t n) {
    if (n >= 32) {
        v16 a = LOAD(s), b = LOAD(s + 16), c = LOAD(s + n - 32), e = LOAD(s + n - 16);
        STORE(d, a); STORE(d + 16, b); STORE(d + n - 32, c); STORE(d + n - 16, e);
    } else if (n >= 16) {
        v16 a = LOAD(s), b = LOAD(s + n - 16);
        STORE(d, a); STORE(d + n - 16, b);
    } else if (n >= 8) {
        u64 a = *(const u64u *)s, b = *(const u64u *)(s + n - 8);
        *(u64u *)d = a; *(u64u *)(d + n - 8) = b;
    } else if (n >= 4) {
        u32 a = *(const u32u *)s, b = *(const u32u *)(s + n - 4);
        *(u32u *)d = a; *(u32u *)(d + n - 4) = b;
    } else if (n >= 2) {
        u16 a = *(const u16u *)s, b = *(const u16u *)(s + n - 2);
        *(u16u *)d = a; *(u16u *)(d + n - 2) = b;
    } else if (n) {
        *d = *s;
    }
}

/* 65 to 128 bytes, loads before stores as above. */
INLINE void copy_medium(u8 *d, const u8 *s, size_t n) {
    v16 a = LOAD(s), b = LOAD(s + 16), c = LOAD(s + 32), e = LOAD(s + 48);
    v16 f = LOAD(s + n - 64), g = LOAD(s + n - 48), h = LOAD(s + n - 32), i = LOAD(s + n - 16);
    STORE(d, a); STORE(d + 16, b); STORE(d + 32, c); STORE(d + 48, e);
    STORE(d + n - 64, f); STORE(d + n - 48, g); STORE(d + n - 32, h); STORE(d + n - 16, i);
}

/* Over 128 bytes, from the start: 64 bytes per step to aligned destination
 * addresses; the first 16 and last 64 bytes are loaded up front and stored
 * last. Also right when the destination overlaps below the source: a step
 * only overwrites source bytes that earlier steps have read. */
INLINE void copy_forward(u8 *d, const u8 *s, size_t n) {
    v16 head = LOAD(s);
    v16 t0 = LOAD(s + n - 64), t1 = LOAD(s + n - 48), t2 = LOAD(s + n - 32), t3 = LOAD(s + n - 16);
    u8 *end = d + n - 64;
    u8 *p = (u8 *)(((u64)d + 16) & ~15UL);
    const u8 *q = s + (p - d);
    while (p < end) {
        v16 x0 = LOAD(q), x1 = LOAD(q + 16), x2 = LOAD(q + 32), x3 = LOAD(q + 48);
        STORE_ALIGNED(p, x0); STORE_ALIGNED(p + 16, x1); STORE_ALIGNED(p + 32, x2); STORE_ALIGNED(p + 48, x3);
        p += 64;
        q += 64;
    }
    STORE(end, t0); STORE(end + 16, t1); STORE(end + 32, t2); STORE(end + 48, t3);
    STORE(d, head);
}

/* The same from the end: right when the destination overlaps above the
 * source, and free of 4K aliasing when it is a little above it. */
INLINE void copy_backward(u8 *d, const u8 *s, size_t n) {
    v16 h0 = LOAD(s), h1 = LOAD(s + 16), h2 = LOAD(s + 32), h3 = LOAD(s + 48);
    v16 tail = LOAD(s + n - 16);
    u8 *p = (u8 *)((u64)(d + n) & ~15UL);
    const u8 *q = s + (p - d);
    while (p > d + 64) {
        v16 x0 = LOAD(q - 16), x1 = LOAD(q - 32), x2 = LOAD(q - 48), x3 = LOAD(q - 64);
        STORE_ALIGNED(p - 16, x0); STORE_ALIGNED(p - 32, x1); STORE_ALIGNED(p - 48, x2); STORE_ALIGNED(p - 64, x3);
        p -= 64;
        q -= 64;
    }
    STORE(d, h0); STORE(d + 16, h1); STORE(d + 32, h2); STORE(d + 48, h3);
    STORE(d + n - 16, tail);
}

__attribute__((no_builtin)) static void *fast_memmove(void *dst, const void *src, size_t n) {
    u8 *d = dst;
    const u8 *s = src;
    if (n <= 64) {
        copy_small(d, s, n);
        return dst;
    }
    if (n <= 128) {
        copy_medium(d, s, n);
        return dst;
    }
    size_t above = (size_t)(d - s); /* destination - source, modulo 2^64 */
    if (above < n) {
        /* Destination overlaps above the source: copy from the end. */
        copy_backward(d, s, n);
        return dst;
    }
    size_t alias = above & 4095;
    if (n >= REP_MOVSB_MIN && n <= REP_MOVSB_MAX && (alias >= ALIAS_WINDOW || !(alias & 31))) {
        /* Forward, also right when the destination overlaps below the source. */
        __asm__ volatile("rep movsb" : "+D"(d), "+S"(s), "+c"(n) : : "memory");
        return dst;
    }
    if (alias < ALIAS_WINDOW && (size_t)(s - d) >= n) {
        /* No overlap, destination a little above the source modulo 4 KB. */
        copy_backward(d, s, n);
        return dst;
    }
    copy_forward(d, s, n);
    return dst;
}
DYLD_INTERPOSE(fast_memmove, memmove)

__attribute__((no_builtin)) static void *fast_memcpy(void *dst, const void *src, size_t n) {
    return fast_memmove(dst, src, n);  /* Darling's memcpy is memmove too */
}
DYLD_INTERPOSE(fast_memcpy, memcpy)

/* `v` (16 copies of one byte) over n > 16 bytes. */
INLINE void fill(u8 *d, v16 v, u8 byte, size_t n) {
    if (n <= 32) {
        STORE(d, v); STORE(d + n - 16, v);
        return;
    }
    if (n <= 64) {
        STORE(d, v); STORE(d + 16, v); STORE(d + n - 32, v); STORE(d + n - 16, v);
        return;
    }
    if (n >= REP_STOSB_MIN && n <= REP_STOSB_MAX) {
        __asm__ volatile("rep stosb" : "+D"(d), "+c"(n) : "a"(byte) : "memory");
        return;
    }
    u8 *end = d + n - 64;
    STORE(d, v);
    u8 *p = (u8 *)(((u64)d + 16) & ~15UL);
    while (p < end) {
        STORE_ALIGNED(p, v); STORE_ALIGNED(p + 16, v); STORE_ALIGNED(p + 32, v); STORE_ALIGNED(p + 48, v);
        p += 64;
    }
    STORE(end, v); STORE(end + 16, v); STORE(end + 32, v); STORE(end + 48, v);
}

__attribute__((no_builtin)) static void *fast_memset(void *dst, int value, size_t n) {
    u8 *d = dst;
    u8 byte = (u8)value;
    u64 word = 0x0101010101010101UL * byte;
    if (n <= 16) {
        if (n >= 8) {
            *(u64u *)d = word; *(u64u *)(d + n - 8) = word;
        } else if (n >= 4) {
            *(u32u *)d = (u32)word; *(u32u *)(d + n - 4) = (u32)word;
        } else if (n) {
            d[0] = byte; d[n - 1] = byte; d[n >> 1] = byte;
        }
        return dst;
    }
    fill(d, (v16)(u64x2){word, word}, byte, n);
    return dst;
}
DYLD_INTERPOSE(fast_memset, memset)

__attribute__((no_builtin)) static void fast_bzero(void *dst, size_t n) {
    fast_memset(dst, 0, n);
}
DYLD_INTERPOSE(fast_bzero, bzero)

/* memset_pattern4/8/16: `pattern` (the 16-byte form of it) repeated over n
 * bytes, the last copy cut short. Darling's call its memmove per copy. */
INLINE void fill_pattern(u8 *d, v16 pattern, size_t n) {
    for (; n >= 64; d += 64, n -= 64) {
        STORE(d, pattern); STORE(d + 16, pattern); STORE(d + 32, pattern); STORE(d + 48, pattern);
    }
    for (; n >= 16; d += 16, n -= 16)
        STORE(d, pattern);
    u64 bytes = ((u64x2)pattern)[0];
    if (n >= 8) {
        *(u64u *)d = bytes;
        bytes = ((u64x2)pattern)[1];
        d += 8;
        n -= 8;
    }
    if (n >= 4) {
        *(u32u *)d = (u32)bytes;
        bytes >>= 32;
        d += 4;
        n -= 4;
    }
    if (n >= 2) {
        *(u16u *)d = (u16)bytes;
        bytes >>= 16;
        d += 2;
        n -= 2;
    }
    if (n)
        *d = (u8)bytes;
}

__attribute__((no_builtin)) static void fast_memset_pattern4(void *dst, const void *pattern, size_t n) {
    u32 word = *(const u32u *)pattern;
    fill_pattern(dst, (v16)(u32x4){word, word, word, word}, n);
}
DYLD_INTERPOSE(fast_memset_pattern4, memset_pattern4)

__attribute__((no_builtin)) static void fast_memset_pattern8(void *dst, const void *pattern, size_t n) {
    u64 word = *(const u64u *)pattern;
    fill_pattern(dst, (v16)(u64x2){word, word}, n);
}
DYLD_INTERPOSE(fast_memset_pattern8, memset_pattern8)

__attribute__((no_builtin)) static void fast_memset_pattern16(void *dst, const void *pattern, size_t n) {
    fill_pattern(dst, LOAD(pattern), n);
}
DYLD_INTERPOSE(fast_memset_pattern16, memset_pattern16)

__attribute__((no_builtin)) static int fast_memcmp(const void *a, const void *b, size_t n) {
    const u8 *x = a, *y = b;
    while (n >= 16) {
        v16 p = LOAD(x), q = LOAD(y);
        if (equal_mask(p, q) != 0xffff)
            break;
        x += 16; y += 16; n -= 16;
    }
    while (n >= 8 && *(const u64u *)x == *(const u64u *)y) {
        x += 8; y += 8; n -= 8;
    }
    for (; n; n--, x++, y++)
        if (*x != *y)
            return (int)*x - (int)*y;
    return 0;
}
DYLD_INTERPOSE(fast_memcmp, memcmp)

/* memchr and strlen read whole aligned 16-byte blocks, or 64-byte groups of
 * them from a 64-byte boundary: such a read never crosses into the next
 * page, so it cannot fault even when the string, or the match, ends at the
 * first byte of the block. memchr also stops at a match as if it read byte
 * by byte (callers may pass a size larger than the buffer). */

__attribute__((no_builtin)) static void *fast_memchr(const void *p, int value, size_t n) {
    const u8 *s = p;
    if (!n)
        return 0;
    u64 word = 0x0101010101010101UL * (u8)value;
    v16 needle = (v16)(u64x2){word, word};
    const u8 *block = (const u8 *)((u64)s & ~15UL);
    u32 skip = (u32)((u64)s & 15);
    u32 mask = equal_mask(*(const v16a *)block, needle) >> skip;
    if (mask)
        return lowest(mask) < n ? (void *)(s + lowest(mask)) : 0;
    if (n <= 16 - skip)
        return 0;
    n -= 16 - skip; /* bytes left from `block + 16` on */
    block += 16;
    while ((u64)block & 63) {
        mask = equal_mask(*(const v16a *)block, needle);
        if (mask)
            return lowest(mask) < n ? (void *)(block + lowest(mask)) : 0;
        if (n <= 16)
            return 0;
        block += 16;
        n -= 16;
    }
    for (; n > 64; block += 64, n -= 64) {
        const v16a *group = (const v16a *)block;
        v16 hits = (v16)((group[0] == needle) | (group[1] == needle) | (group[2] == needle) | (group[3] == needle));
        if (__builtin_ia32_pmovmskb128(hits))
            break;
    }
    for (;; block += 16, n -= 16) {
        mask = equal_mask(*(const v16a *)block, needle);
        if (mask)
            return lowest(mask) < n ? (void *)(block + lowest(mask)) : 0;
        if (n <= 16)
            return 0;
    }
}
DYLD_INTERPOSE(fast_memchr, memchr)

__attribute__((no_builtin)) static size_t fast_strlen(const char *text) {
    const v16 zero = {0};
    const u8 *block = (const u8 *)((u64)text & ~15UL);
    u32 mask = equal_mask(*(const v16a *)block, zero) >> ((u64)text & 15);
    if (mask)
        return lowest(mask);
    for (block += 16; (u64)block & 63; block += 16) {
        mask = equal_mask(*(const v16a *)block, zero);
        if (mask)
            return (size_t)(block + lowest(mask) - (const u8 *)text);
    }
    for (;; block += 64) {
        const u8x16 *group = (const u8x16 *)block;
        u8x16 least = min_u8(min_u8(group[0], group[1]), min_u8(group[2], group[3]));
        if (equal_mask((v16)least, zero))
            break;
    }
    for (;; block += 16) {
        mask = equal_mask(*(const v16a *)block, zero);
        if (mask)
            return (size_t)(block + lowest(mask) - (const u8 *)text);
    }
}
DYLD_INTERPOSE(fast_strlen, strlen)

/* strcmp/strncmp: unaligned 16-byte loads of both strings while neither can
 * reach into the next page, 64 bytes per step once both are 64 bytes away
 * from a page end; byte by byte past a page end. */
#define ROOM(p, k) (((u64)(p) & 4095) <= 4096 - (k)) /* k bytes readable in this page */

/* Per byte of the next 16: 0 where the strings differ or `x` ends, else
 * nonzero (min of the "equal" mask and x's byte). */
INLINE u8x16 same16(const u8 *x, const u8 *y) {
    v16 p = LOAD(x), q = LOAD(y);
    return min_u8((u8x16)(p == q), (u8x16)p);
}
INLINE u32 stops(u8x16 same) { return equal_mask((v16)same, (v16){0}); }

/* The first stop in the next 64 bytes, or 64. */
INLINE u32 stop64(const u8 *x, const u8 *y) {
    u8x16 s0 = same16(x, y), s1 = same16(x + 16, y + 16), s2 = same16(x + 32, y + 32), s3 = same16(x + 48, y + 48);
    if (!stops(min_u8(min_u8(s0, s1), min_u8(s2, s3))))
        return 64;
    u64 mask = (u64)stops(s0) | (u64)stops(s1) << 16 | (u64)stops(s2) << 32 | (u64)stops(s3) << 48;
    return (u32)__builtin_ctzll(mask);
}

__attribute__((no_builtin)) static int fast_strcmp(const char *a, const char *b) {
    const u8 *x = (const u8 *)a, *y = (const u8 *)b;
    for (u32 steps = 0;; steps++) {
        if (ROOM(x, 16) && ROOM(y, 16)) {
            u32 mask = stops(same16(x, y));
            if (mask)
                return (int)x[lowest(mask)] - (int)y[lowest(mask)];
            x += 16;
            y += 16;
            for (; steps > 1 && ROOM(x, 64) && ROOM(y, 64); x += 64, y += 64) { /* from byte 48 on */
                u32 i = stop64(x, y);
                if (i < 64)
                    return (int)x[i] - (int)y[i];
            }
            continue;
        }
        for (int i = 0; i < 16; i++, x++, y++)
            if (*x != *y || !*x)
                return (int)*x - (int)*y;
    }
}
DYLD_INTERPOSE(fast_strcmp, strcmp)

__attribute__((no_builtin)) static int fast_strncmp(const char *a, const char *b, size_t n) {
    const u8 *x = (const u8 *)a, *y = (const u8 *)b;
    for (u32 steps = 0; n; steps++) {
        if (ROOM(x, 16) && ROOM(y, 16)) {
            u32 mask = stops(same16(x, y));
            if (n < 16)
                mask &= (1u << n) - 1;
            if (mask)
                return (int)x[lowest(mask)] - (int)y[lowest(mask)];
            if (n <= 16)
                return 0;
            x += 16;
            y += 16;
            n -= 16;
            for (; steps > 1 && n >= 64 && ROOM(x, 64) && ROOM(y, 64); x += 64, y += 64, n -= 64) {
                u32 i = stop64(x, y);
                if (i < 64)
                    return (int)x[i] - (int)y[i];
            }
            continue;
        }
        for (int i = 0; i < 16 && n; i++, x++, y++, n--)
            if (*x != *y || !*x)
                return (int)*x - (int)*y;
    }
    return 0;
}
DYLD_INTERPOSE(fast_strncmp, strncmp)
