//! Deterministic corpus generation (issue #19 §7): Corpus B (controlled
//! synthetic documents) and Corpus C (adversarial documents).
//!
//! Corpus A (CommonMark spec examples) is a later addition; it needs the
//! spec test file and belongs with the correctness-oracle expansion.

pub struct SynthOptions {
    pub target_bytes: usize,
    pub cjk: bool,
    pub crlf: bool,
}

const HEADER_LATIN: &str = "\
# Markit survey corpus

Intro paragraph with *emphasis*, a `code span`, and a [link](/target).

**Bold-led paragraph with *nested* emphasis and `code`.**

[surveyref]: /survey/target

- alpha item
- beta item
  - gamma nested item
- delta item
lazy continuation of delta item

> quote line alpha
> quote line beta
lazy quote continuation

```rust
fn sample() -> u32 { 42 }
```

```mermaid
graph TD; A --> B;
```

";

const HEADER_CJK: &str = "\
# 调研语料标题

开头段落包含*强调*、`代码片段`和一个 [链接](/target)。

**粗体开头的段落包含*嵌套强调*和`代码`。**

[surveyref]: /survey/target

- 甲项内容
- 乙项内容
  - 丙项嵌套内容
- 丁项内容
丁项的懒惰续行

> 引用第一行
> 引用第二行
引用的懒惰续行

```rust
fn sample() -> u32 { 42 }
```

```mermaid
graph TD; A --> B;
```

";

/// Builds a document of approximately `target_bytes` bytes: one kitchen
/// sink header (every L1 construct, including a mermaid fence), then
/// repeated two-line paragraphs. Deterministic.
pub fn synthetic(opts: SynthOptions) -> String {
    let mut s = String::with_capacity(opts.target_bytes.saturating_mul(2).max(4096));
    if opts.cjk {
        s.push_str(HEADER_CJK);
    } else {
        s.push_str(HEADER_LATIN);
    }
    let mut k = 0u64;
    while s.len() < opts.target_bytes {
        k += 1;
        if opts.cjk {
            s.push_str(&format!(
                "第{k}段第一行包含若干汉字与数字{k}用于测试解析器的行为表现。\n第{k}段第二行继续同一个段落块并且包含更多的汉字内容。\n\n"
            ));
        } else {
            s.push_str(&format!(
                "Paragraph {k} line one carries several ordinary words for the survey corpus.\nParagraph {k} line two continues the same block with more ordinary words.\n\n"
            ));
        }
    }
    if opts.crlf {
        s = s.replace('\n', "\r\n");
    }
    s
}

#[derive(Clone, Copy)]
pub enum Adv {
    UnclosedFence,
    DeepQuote,
    LazyContinuation,
    HugeParagraph,
    ManyRefs,
    DuplicateRefs,
    CrlfMixed,
    HalfWritten,
    FenceNearBof,
    Emoji,
}

/// Adversarial documents, each attacking one part of the taxonomy
/// (unclosed fences, deep containers, lazy continuation, setext-scale
/// ambiguity, reference fanout, CRLF, half-written Markdown).
pub fn adversarial(kind: Adv) -> (String, &'static str) {
    match kind {
        Adv::UnclosedFence => (
            format!(
                "Para before the fence.\n\n```rust\nlet x = 1;\n{}\n",
                "let y = 2;\n".repeat(50)
            ),
            "adv-unclosed-fence",
        ),
        Adv::DeepQuote => {
            let mut s = String::new();
            for i in 1..=200 {
                s.push_str(&"> ".repeat(i));
                s.push_str(&format!("depth {i}\n"));
            }
            s.push_str("after paragraph\n");
            (s, "adv-deep-quote")
        }
        Adv::LazyContinuation => (
            "# Head\n\n> quoted line one\nlazy continuation line one\n> quoted line two\nsecond lazy continuation\n\nplain paragraph tail\n"
                .to_string(),
            "adv-lazy-continuation",
        ),
        Adv::HugeParagraph => (
            format!(
                "before paragraph\n\n{}\n\nafter paragraph\n",
                "word ".repeat(20000)
            ),
            "adv-huge-paragraph",
        ),
        Adv::ManyRefs => {
            let mut s = String::from("Uses [r1] and [r2] and [r500] inline here.\n\n");
            for i in 1..=1000 {
                s.push_str(&format!("[r{i}]: /url/{i}\n"));
            }
            (s, "adv-many-refs")
        }
        Adv::DuplicateRefs => (
            "[x]: /first\n\nText using [x] once here.\n\nMore text using [x] again.\n\n[x]: /second\n"
                .to_string(),
            "adv-duplicate-refs",
        ),
        Adv::CrlfMixed => (
            "# Head\r\n\r\nCRLF paragraph one.\r\n\r\nLF only paragraph two.\n\r\nCRLF paragraph three.\r\n"
                .to_string(),
            "adv-crlf-mixed",
        ),
        Adv::HalfWritten => (
            "# Half written doc\n\n**bold never closed\n\n`` `weird span\n\n[dead link](/url\n\n- item one\n- item two\n"
                .to_string(),
            "adv-half-written",
        ),
        Adv::FenceNearBof => (
            "```\nbody line one\nbody line two\n```\n\nafter paragraph\n".to_string(),
            "adv-fence-near-bof",
        ),
        // ORACLE-C (losslessness) corpus: U3 on the Unicode ladder —
        // BMP emoji, surrogate-pair astral emoji, flag sequences (regional
        // indicators), ZWJ family and skin-tone sequences, combining
        // marks, across heading/paragraph/quote/list/fence constructs.
        Adv::Emoji => (
            "\
# Emoji 语料 😀 heading with astral 👨‍👩‍👧‍👦 family

Paragraph with 👍🏻 thumbs-up skin tone, 🇺🇸 flag, é combining acute, \
and mixed CJK 汉字 🎉 end.

> quote line with 🚀 rocket and ZWJ 👩‍💻 woman technologist

- item with ❤️ heart
- item with 👱🏽‍♀️ andcaf\u{301}é

```text
fence body with 😀 raw surrogate-pair emoji
```

Trailing paragraph 😄
"
            .to_string(),
            "adv-emoji",
        ),
    }
}

pub fn human_size(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{}m", bytes / (1024 * 1024))
    } else if bytes >= 1024 {
        format!("{}k", bytes / 1024)
    } else {
        format!("{bytes}b")
    }
}
