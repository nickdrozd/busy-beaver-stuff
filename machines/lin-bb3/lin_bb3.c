// lin_bb3.c
// Reproduce Shen Lin (1963) BB-3 (3-card binary) normalized search pipeline.
//
// This single-file program implements:
//   1) Lin's normalized enumeration: 4 lots x 12^4 = 82,944 machines.
//      Fixed lines: Card1-0 = 112, and the unique stop-line = 110.
//   2) Discard machines that stop in <= 21 shifts (recording scores).
//   3) Lin's "obvious" pruning rules for some lots.
//   4) Lin's PARTIAL RECURRENCE routine (36-bit tape word, start square at bit 18)
//      exactly as described in Chapter III.
//   5) Print remaining "holdouts" in standard TM program notation:
//         A0 A1  B0 B1  C0 C1
//      e.g. 1RB 1RH  0LC 0RA  1LA 1LB
//
// Build:
//   gcc -O3 -std=c11 -Wall -Wextra lin_bb3.c -o lin_bb3
// Run:
//   ./lin_bb3
//
// References (from Lin dissertation PDF):
// - Normalization to 82,944 and lots, stop<=21 phase: see Chapter III. fileciteturn16file2L13-L22
// - Partial recurrence routine formulas and 50-shift bound, spill check: fileciteturn16file3L1-L20
// - Barrier intuition: compare tape between barriers / drifting recurrence: fileciteturn16file0L11-L15

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define NUM_LINES 6
#define NUM_LOTS 4
#define MAX_STOP_SCAN 21
#define MAX_REC_SHIFTS 50

// Full tape for accurate scoring in the <=21 shift scan
#define TAPE_SIZE 4096
#define TAPE_MID  (TAPE_SIZE/2)

// 36-bit tape word used by Lin's recurrence routine
#define WORD_BITS 36
#define START_BIT 18          // starting square at bit 18 fileciteturn16file0L18-L19
#define DEV_LIMIT 17          // spill when |deviation| > 17 fileciteturn16file3L11-L14
#define WORD_MASK ((uint64_t)((1ULL<<WORD_BITS)-1ULL))

// --- Bit-numbering conventions (compile-time tunable) ---
// Lin's text uses expressions like "T shifted left 18 + D bits" fileciteturn16file3L1-L3.
// Different machines/notations may number bits MSB->LSB or LSB->MSB.
// To reproduce Lin's counts, we keep the mapping explicit:
//   - deviation d corresponds to bit position BITPOS(d) within the 36-bit word.
//   - the "shift left" operation used in comparisons is SHIFT36(word, k).

#ifndef BITPOS
// Default: bit index = START_BIT + deviation (so D=0 is bit 18)
#define BITPOS(d) (START_BIT + (d))
#endif

#ifndef SHIFT36
// Default: C-style logical left shift within 36-bit width
#define SHIFT36(word, k) (shl36((word), (k)))
#endif

#ifndef PRINT_HOLDOUTS
#define PRINT_HOLDOUTS 1
#endif

#ifndef SHIFT36
#define SHIFT36(w, k) shift_left36((w), (k))
#endif

static inline int line_index(int card /*1..3*/, int sym /*0/1*/) {
    return (card-1)*2 + sym;
}

// Lin's 4-bit line encoding: [p][s][c1][c0]
static inline uint8_t enc_line(uint8_t p, uint8_t s, uint8_t c) {
    return (uint8_t)((p<<3) | (s<<2) | (c & 3));
}
static inline uint8_t get_p(uint8_t w) { return (w>>3)&1; }
static inline uint8_t get_s(uint8_t w) { return (w>>2)&1; }
static inline uint8_t get_c(uint8_t w) { return w & 3; }

// 12 possible non-stop cases for a free line: p∈{0,1}, s∈{0,1}, c∈{1,2,3}
static void gen_12_cases(uint8_t cases12[12]) {
    int t = 0;
    for (uint8_t p=0;p<=1;p++) {
        for (uint8_t s=0;s<=1;s++) {
            for (uint8_t c=1;c<=3;c++) {
                cases12[t++] = enc_line(p,s,c);
            }
        }
    }
}

// Build normalized machine for a given lot, with 4 free lines
static void build_machine_for_lot(int lot, const uint8_t free4[4], uint8_t out[NUM_LINES]) {
    // initialize with a placeholder non-stop line
    for (int i=0;i<NUM_LINES;i++) out[i] = enc_line(0,0,1);

    // fixed Card1-0 line = 112
    out[0] = enc_line(1,1,2);

    // determine stop-line index per lot
    // Lot1: Card1-1
    // Lot2: Card2-1
    // Lot3: Card3-0
    // Lot4: Card3-1
    int stop_idx = -1;
    if (lot == 1) stop_idx = 1;
    if (lot == 2) stop_idx = 3;
    if (lot == 3) stop_idx = 4;
    if (lot == 4) stop_idx = 5;

    // stop-line fixed to 110
    out[stop_idx] = enc_line(1,1,0);

    // assign remaining 4 lines in deterministic order:
    // all indices except 0 and stop_idx
    int k = 0;
    for (int i=0;i<NUM_LINES;i++) {
        if (i==0 || i==stop_idx) continue;
        out[i] = free4[k++];
    }
}

// Lin's "obvious" pruning rules
// Lot1: discard if no call to Card1 appears in Cards 2 and 3
// Lots3&4: discard if no call to Card3 appears in Cards 1 and 2
// (as stated in Chapter III results discussion) fileciteturn10file0L365-L366
static int prune_obvious(int lot, const uint8_t lines[NUM_LINES]) {
    if (lot == 1) {
        // among Card2/3 lines (idx2..5), check any c==1
        for (int i=2;i<=5;i++) if (get_c(lines[i]) == 1) return 0;
        return 1;
    }
    if (lot == 3 || lot == 4) {
        // among Card1-1 (idx1) and Card2 lines (idx2,idx3), check any c==3
        if (get_c(lines[1]) == 3) return 0;
        if (get_c(lines[2]) == 3) return 0;
        if (get_c(lines[3]) == 3) return 0;
        return 1;
    }
    return 0;
}

// --- TM program notation printer ---
// Card1=A, Card2=B, Card3=C, stop=H
static char state_letter(uint8_t c) {
    if (c==0) return 'H';
    if (c==1) return 'A';
    if (c==2) return 'B';
    return 'C';
}

static void line_to_tm(uint8_t w, char out[4]) {
    // out: e.g. "1RB"
    out[0] = (char)('0' + get_p(w));
    out[1] = get_s(w) ? 'R' : 'L';
    out[2] = state_letter(get_c(w));
    out[3] = '\0';
}

/* Pack Lin’s 6 line nibbles into a 24-bit word.
   lines[] must be in order: A0,A1,B0,B1,C0,C1.
*/
static inline uint32_t lin_word24_from_lines(const uint8_t lines[6]) {
    uint32_t w = 0;
    w |= (uint32_t)(lines[0] & 0xF) << 0;   // A0
    w |= (uint32_t)(lines[1] & 0xF) << 4;   // A1
    w |= (uint32_t)(lines[2] & 0xF) << 8;   // B0
    w |= (uint32_t)(lines[3] & 0xF) << 12;  // B1
    w |= (uint32_t)(lines[4] & 0xF) << 16;  // C0
    w |= (uint32_t)(lines[5] & 0xF) << 20;  // C1
    return w;
}

/* Print Lin serial number as 8 octal digits (leading zeros). */
static inline void print_lin_serial_octal_from_lines(const uint8_t lines[6]) {
    uint32_t w24 = lin_word24_from_lines(lines);
    printf("%08o", w24);   // 24-bit word → 8 octal digits
}

static void print_machine_tm(const uint8_t lines[NUM_LINES]) {
    printf("Serial=");
    print_lin_serial_octal_from_lines(lines);
    printf("  ");
    char a0[4], a1[4], b0[4], b1[4], c0[4], c1[4];
    line_to_tm(lines[0], a0);
    line_to_tm(lines[1], a1);
    line_to_tm(lines[2], b0);
    line_to_tm(lines[3], b1);
    line_to_tm(lines[4], c0);
    line_to_tm(lines[5], c1);
    printf("%s %s  %s %s  %s %s", a0,a1,b0,b1,c0,c1);
}

// --- Phase 1: run machine up to 21 shifts, accurate score on full tape ---
// STOP line halts after executing its print+shift (Lin fixes stop-line to 110).
// Score = number of 1s on tape at stop.
static int tape_score(const uint8_t *tape, int min_i, int max_i) {
    int s = 0;
    for (int i=min_i;i<=max_i;i++) s += (tape[i] != 0);
    return s;
}

typedef struct {
    int stopped; // 1 if stopped within bound
    int shifts;
    int score;
} StopScanResult;

static StopScanResult run_stop_scan_21(const uint8_t lines[NUM_LINES]) {
    uint8_t tape[TAPE_SIZE];
    memset(tape, 0, sizeof(tape));
    int head = TAPE_MID;
    int card = 1;
    int minTape = head, maxTape = head;

    for (int s=1; s<=MAX_STOP_SCAN; s++) {
        int scanned = tape[head] & 1;
        uint8_t w = lines[line_index(card, scanned)];

        // execute print
        tape[head] = get_p(w);
        if (head < minTape) minTape = head;
        if (head > maxTape) maxTape = head;

        // execute shift
        if (get_s(w)) head++; else head--;
        if (head < 0 || head >= TAPE_SIZE) {
            StopScanResult r = {0, s, 0};
            return r;
        }

        // stop?
        if (get_c(w) == 0) {
            StopScanResult r;
            r.stopped = 1;
            r.shifts = s;
            r.score = tape_score(tape, minTape, maxTape);
            return r;
        }

        card = get_c(w);
    }

    StopScanResult r = {0, MAX_STOP_SCAN, 0};
    return r;
}

// --- Lin's 36-bit recurrence routine implementation ---

// 36-bit shift-left with zero fill, keeping 36-bit width
static inline uint64_t shl36(uint64_t x, int k) {
    if (k <= 0) return x & WORD_MASK;
    if (k >= WORD_BITS) return 0ULL;
    return (x << k) & WORD_MASK;
}

// 36-bit shift-right with zero fill (for alternative bit-numbering conventions)
static inline uint64_t shr36(uint64_t x, int k) {
    if (k <= 0) return x & WORD_MASK;
    if (k >= WORD_BITS) return 0ULL;
    return (x >> k) & WORD_MASK;
}

static inline int bit_at(uint64_t T, int dev) {
    // return tape bit at square with deviation dev (within [-DEV_LIMIT..DEV_LIMIT])
    int bp = BITPOS(dev);
    if (bp < 0 || bp >= WORD_BITS) return 0; // outside tracked word treated as 0
    return (int)((T >> bp) & 1ULL);
}

// Compare tape segments for Lin's barrier recurrence logic (see discussion preceding routine)
// For Dq < D (current head is to the right): compare the portion of tape to the right of the
// left barrier (minimum deviation Dmin) with the earlier pattern shifted by delta = D - Dq.
static int compare_right_of_left_barrier(uint64_t Tq, uint64_t T, int Dmin, int delta) {
    // Compare dev in [Dmin .. DEV_LIMIT - delta] : Tq[dev] == T[dev + delta]
    int start = Dmin;
    int end = DEV_LIMIT - delta;
    if (end < start) return 0;
    for (int dev = start; dev <= end; dev++) {
        if (bit_at(Tq, dev) != bit_at(T, dev + delta)) return 0;
    }
    return 1;
}

// For Dq > D (current head is to the left): compare the portion of tape to the left of the
// right barrier (maximum deviation Dmax) with the earlier pattern shifted by delta = D - Dq (negative).
static int compare_left_of_right_barrier(uint64_t Tq, uint64_t T, int Dmax, int delta) {
    // delta < 0. Compare dev in [(-DEV_LIMIT - delta) .. Dmax] : Tq[dev] == T[dev + delta]
    int start = -DEV_LIMIT - delta;
    if (start < -DEV_LIMIT) start = -DEV_LIMIT;
    int end = Dmax;
    if (end < start) return 0;
    for (int dev = start; dev <= end; dev++) {
        if (bit_at(Tq, dev) != bit_at(T, dev + delta)) return 0;
    }
    return 1;
}

static inline uint64_t mask_range_bits(int lo, int hi) {
    // inclusive, within [0..35]
    if (lo < 0) lo = 0;
    if (hi > WORD_BITS-1) hi = WORD_BITS-1;
    if (hi < lo) return 0ULL;
    int len = hi - lo + 1;
    if (len >= WORD_BITS) return WORD_MASK;
    uint64_t m = (len == 64) ? ~0ULL : ((1ULL << len) - 1ULL);
    return (m << lo) & WORD_MASK;
}

typedef struct {
    uint64_t T;
    int S;
    int D;
} TBEntry;

typedef enum {
    REC_LOOPED = 1,
    REC_NO_RECURRENCE = 0,
    REC_SPILL = -1,
    REC_STOPPED = 2
} RecResult;

// Compute min/max deviation between shifts Sq and s inclusive
static inline void dev_minmax(const int dev[], int Sq, int s, int *outMin, int *outMax) {
    int mn = dev[Sq];
    int mx = dev[Sq];
    for (int k=Sq; k<=s; k++) {
        if (dev[k] < mn) mn = dev[k];
        if (dev[k] > mx) mx = dev[k];
    }
    *outMin = mn;
    *outMax = mx;
}

// Lin recurrence routine: run up to 50 shifts looking for partial recurrence.
// Returns:
//  - REC_LOOPED if recurrence detected => discard never-stopper
//  - REC_NO_RECURRENCE if none within 50 => holdout
//  - REC_SPILL if |deviation|>17 => holdout (spilled beyond 36-bit word)
//  - REC_STOPPED if it stops (should not happen if SH(3)=21)
static RecResult run_lin_recurrence_50(const uint8_t lines[NUM_LINES]) {
    // tape word bits: bit(BITPOS(D)) corresponds to square at deviation D
    uint64_t T = 0ULL;
    int D = 0;      // deviation of head relative to starting square
    int card = 1;

    // deviation history (after each shift) dev[s] = D
    int dev[MAX_REC_SHIFTS+1];
    dev[0] = 0;

    // Tape tables TB[i][j], i=1..3, j=0..1
    TBEntry tb[4][2][MAX_REC_SHIFTS+1];
    int tbCount[4][2];
    memset(tbCount, 0, sizeof(tbCount));

    // We begin before shift 1 with all-0 tape; scanned digit at start is 0.

    for (int s=1; s<=MAX_REC_SHIFTS; s++) {
        // scanned symbol at current head (deviation D)
        if (D < -DEV_LIMIT || D > DEV_LIMIT) {
            return REC_SPILL;
        }
        int bitpos = BITPOS(D);
        int scanned = (int)((T >> bitpos) & 1ULL);

        // execute current instruction
        uint8_t w = lines[line_index(card, scanned)];
        uint8_t p = get_p(w);
        uint8_t sh = get_s(w);
        uint8_t c = get_c(w);

        // print: set bit at current deviation
        if (p) T |= (1ULL << bitpos);
        else   T &= ~(1ULL << bitpos);

        // shift head
        if (sh) D++; else D--;

        // stop?
        if (c == 0) {
            dev[s] = D;
            return REC_STOPPED;
        }

        // call next card
        card = (int)c;

        // spill check (after shift)
        dev[s] = D;
        if (D < -DEV_LIMIT || D > DEV_LIMIT) {
            return REC_SPILL;
        }

        // scanned digit after shift, used to index TB[card][j]
        int bitpos2 = BITPOS(D);
        int j = (int)((T >> bitpos2) & 1ULL);

        // insert into tape table TB[card][j]
        int cnt = tbCount[card][j];

        // if table nonempty, test against previous entries
        for (int q=0; q<cnt; q++) {
            TBEntry *e = &tb[card][j][q];
            uint64_t Tq = e->T;
            int Sq = e->S;
            int Dq = e->D;

            if (Dq < D) {
                // Dq < D: find Dmin between Sq and s, then compare shifted words
                int Dmin, Dmax;
                dev_minmax(dev, Sq, s, &Dmin, &Dmax);

                // Tq shifted left 18 + Dq bits
                // T  shifted left 18 + Dmin + D - Dq bits
                // (Lin: "Tq is shifted left 18 + Dq bits and T shifted left 18 + Dmin + D - Dq bits")
                // fileciteturn16file3L1-L3
                // Lin's OCR scan truncates the symbol after "18 +" in some copies.
                // The barrier-based derivation implies shifting relative to the barrier
                // (minimum deviation) rather than the earlier endpoint deviation.
                int delta = D - Dq;
                if (compare_right_of_left_barrier(Tq, T, Dmin, delta)) {
                    return REC_LOOPED;
                }

            } else if (Dq > D) {
                // symmetric when Dq > D
                int Dmin, Dmax;
                dev_minmax(dev, Sq, s, &Dmin, &Dmax);

                // symmetric right-barrier analogue: use Dmax instead of Dmin
                // (Lin: "Symmetrical procedure hold when Dq > D") fileciteturn16file3L8
                int delta = D - Dq; // negative
                if (compare_left_of_right_barrier(Tq, T, Dmax, delta)) {
                    return REC_LOOPED;
                }

            } else {
                // Dq == D: use both barriers (mask compare between barriers)
                // Lin: "If Dq = D, both Dmax and Dmin are determined and Tq and T
                // are compared from bits ... to ... by the use of a mask." fileciteturn16file3L9-L10
                int Dmin, Dmax;
                dev_minmax(dev, Sq, s, &Dmin, &Dmax);

                int lo = BITPOS(Dmin);
                int hi = BITPOS(Dmax);
                uint64_t m = mask_range_bits(lo, hi);
                if ( (Tq & m) == (T & m) ) {
                    return REC_LOOPED;
                }
            }
        }

        // no recurrence found; append entry
        tb[card][j][cnt].T = T;
        tb[card][j][cnt].S = s;
        tb[card][j][cnt].D = D;
        tbCount[card][j] = cnt + 1;

        // continue to next shift
    }

    // no recurrence after 50 shifts => holdout fileciteturn16file3L18-L20
    return REC_NO_RECURRENCE;
}

int main(void) {
    uint8_t cases12[12];
    gen_12_cases(cases12);

    int total = 0;
    int stoppers = 0;
    int bestScore = -1;
    int bestScoreShifts = 0;
    uint8_t bestScoreMachine[NUM_LINES];
    int bestShifts = -1;
    int bestShiftsScore = 0;
    uint8_t bestShiftMachine[NUM_LINES];

    int candidates = 0;
    int obviousPruned = 0;
    int recLooped = 0;
    int holdouts = 0;
    int spilled = 0;
    int stoppedBeyond21 = 0;

    printf("Lin BB-3 normalized enumeration: 4 lots x 12^4 = 82,944 machines\n");
    printf("Phase 1: discard machines that stop in <= %d shifts\n", MAX_STOP_SCAN);

    for (int lot=1; lot<=NUM_LOTS; lot++) {
        int lotTotal=0, lotStop=0, lotCand=0, lotPrune=0, lotHold=0;

        for (int a=0;a<12;a++)
        for (int b=0;b<12;b++)
        for (int c=0;c<12;c++)
        for (int d=0;d<12;d++) {
            uint8_t free4[4] = {cases12[a], cases12[b], cases12[c], cases12[d]};
            uint8_t m[NUM_LINES];
            build_machine_for_lot(lot, free4, m);

            total++; lotTotal++;

            StopScanResult r = run_stop_scan_21(m);
            if (r.stopped) {
                stoppers++; lotStop++;

                if (r.score > bestScore) {
                    bestScore = r.score;
                    bestScoreShifts = r.shifts;
                    memcpy(bestScoreMachine, m, NUM_LINES);
                }
                if (r.shifts > bestShifts) {
                    bestShifts = r.shifts;
                    bestShiftsScore = r.score;
                    memcpy(bestShiftMachine, m, NUM_LINES);
                }

                // Lin printed champions score>=6 or shifts>=20
                if (r.score >= 6 || r.shifts >= 20) {
                    printf("HALTED  stop@%2d score=%d lot=%d :: ", r.shifts, r.score, lot);
                    print_machine_tm(m);
                    printf("\n");
                }
                continue;
            }

            // Not stopped within 21 shifts
            candidates++; lotCand++;

            if (prune_obvious(lot, m)) {
                obviousPruned++; lotPrune++;
                continue;
            }

            // Lin recurrence routine
            RecResult rr = run_lin_recurrence_50(m);
            if (rr == REC_LOOPED) {
                recLooped++;
            } else if (rr == REC_STOPPED) {
                stoppedBeyond21++;
                // if this ever triggers, SH(3) > 21 (contradicts Lin)
                printf("WARNING: stopper beyond 21 shifts lot=%d :: ", lot);
                print_machine_tm(m);
                printf("\n");
            } else {
                // holdout or spill
                holdouts++; lotHold++;
                if (rr == REC_SPILL) spilled++;

                if (PRINT_HOLDOUTS) {
                    printf("HOLDOUT lot=%d (%s) :: ", lot, (rr==REC_SPILL)?"spill":"no-recurrence");
                    print_machine_tm(m);
                    printf("\n");
                }
            }
        }

        printf("Lot %d: total=%d stoppers<=21=%d candidates=%d pruned=%d holdouts=%d\n",
               lot, lotTotal, lotStop, lotCand, lotPrune, lotHold);
    }

    printf("\n=== SUMMARY ===\n");
    printf("Machines enumerated: %d (expected 82944)\n", total);
    printf("Stoppers (<=21 shifts): %d (Lin reports 26073)\n", stoppers);
    printf("Candidates after 21 shifts: %d\n", candidates);
    printf("Obvious pruned: %d\n", obviousPruned);
    printf("Recurrence-discarded (looped): %d\n", recLooped);
    printf("Holdouts remaining: %d (Lin reports 40)\n", holdouts);
    printf("  of which spills: %d\n", spilled);
    printf("Stopped beyond 21 (should be 0): %d\n", stoppedBeyond21);

    printf("\nBest score observed: %d (expected Sigma(3)=6)\n", bestScore);
    if (bestScore >= 0) {
        printf("  achieved at %d shifts by: ", bestScoreShifts);
        print_machine_tm(bestScoreMachine);
        printf("\n");
    }

    printf("\nMax shifts among stoppers observed: %d (expected SH(3)=21)\n", bestShifts);
    if (bestShifts >= 0) {
        printf("  score at max shifts: %d, machine: ", bestShiftsScore);
        print_machine_tm(bestShiftMachine);
        printf("\n");
    }

    return 0;
}
