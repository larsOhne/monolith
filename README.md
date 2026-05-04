# Monolith

> "Remember — it all depends on who uses it, how they use it, and to what purpose."
> — Yennefer of Vengerberg, *Blood of Elves*

Monolith is a personal knowledge system built around a single idea: **every claim you make should trace back to something real**.

Not a summary. Not a paraphrase. A verbatim passage, in the original source, that you or an AI agent physically marked and said *this is why I believe that*.

---

## The problem with most knowledge tools

Most note-taking systems are great at capturing thoughts. They're bad at remembering *why* you had them.

You write a claim. You link it to a paper. Six months later the paper gets updated, your notes drift, and the connection between what you believe and what the evidence actually says quietly breaks. Nobody notices.

Monolith is designed so that can't happen silently.

---

## Standing on old shoulders

Scholars have always known that a claim without a source is just an opinion. Theologians copied scripture word for word in the margins of their arguments. Scientists footnoted obsessively. Niklas Luhmann built his Zettelkasten out of index cards, each one a single atomic idea linked by hand to every related card in the box. The physical act of marking a passage, copying it out, and connecting it to your own thinking was not busywork. It was the thinking.

Monolith is that practice, rebuilt for the age of documents that change.

---

## How it works

There are three things in Monolith:

**Sources** are the raw documents: papers, articles, books, personal notes. Monolith never modifies them. They stay exactly as you found them.

**Evidence** is a verbatim passage you have pinned inside a source. Not a summary of what it says but the actual words, frozen at the moment you marked them. When the source changes, Monolith notices and tells you exactly what broke and why.

**Statements** are the claims you actually believe. Each one is synthesized from one or more pieces of evidence. A statement without evidence does not exist in Monolith, or at least it cannot pretend to be grounded when it isn't.

The flow is always: *source, then evidence, then statement*. Never the other way around.

---

## The annotation layer

One of Monolith's core commitments is that your source files stay pristine. Annotations, highlights, and validity metadata live in a separate store and are overlaid when you view a document, the same way a translator's notes don't get printed inside the original manuscript.

This means you can open any source in any editor and see exactly what the author wrote, nothing added. The knowledge layer is always clearly separate from the evidence layer.

---

## Git as memory

Monolith uses git to track when sources change. When you pin a passage, Monolith remembers the exact version of the document you were reading. If that source is later edited, Monolith can show you precisely what changed and whether your evidence was affected.

This turns citation into something closer to a contract: *I believe X because this specific version of this document said so.*

---

## Projects and the global wiki

Monolith is designed for people who work across multiple contexts: a research vault, a codebase, a reading list. Each project keeps its own sources and evidence. But all of it feeds into a single global knowledge graph that you can query, navigate, and synthesize across.

When two projects touch the same concept, Monolith does not force them to be the same thing. Instead it lets you explicitly group them under a shared idea, with a note about how each treatment differs. You keep the nuance without losing the connection.

---

## The philosophy in one sentence

A belief you cannot trace to evidence is not knowledge. It is a guess with good formatting.

Monolith is the tool for people who want to know the difference.

---

## What Monolith is not

It is not a replacement for thinking. The agent can find and mark evidence, but only you can decide what it means.

It is not a database of facts. It is a record of your interpretation of sources, with the receipts attached.

It is not trying to be Notion, Roam, or Obsidian. Those tools help you think. Monolith helps you know what you actually know.

---

*Open source. Built on [Graphify](https://github.com/safishamsi/graphify). Inspired by Karpathy's wiki and the long tradition of scholars who underlined things.*
