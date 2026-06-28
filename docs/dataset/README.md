---
license: cc-by-4.0
language:
- en
tags:
- predictions
- technology
- forecasting
- trend-diffusion
- time-series
pretty_name: "THE SIGNAL: Dated, Self-Graded Tech Predictions"
---

# THE SIGNAL: dated, self-graded tech predictions and discourse diffusion

A daily, rules-based (no-LLM) oracle that makes dated, falsifiable technology predictions and grades every one HIT or MISS in public. This dataset is the full public record plus the term-level diffusion data the calls are built on. Live site: https://mattbusel.github.io/tech-oracle/

## Files
- `predictions.csv` / `predictions.jsonl`: every public call (298 so far, 44 settled), with its market type, machine-checkable win condition, confidence, status and resolution date.
- `diffusion.csv`: each tracked term's path down the funnel, from the technical source where it first appeared to the most general audience it has reached, and whether and when it crossed into the general public.
- `datapackage.json`: Frictionless Data descriptor. `croissant.json`: MLCommons/Croissant metadata.

## How it is built
Every day the engine reads ten public sources ordered from technical to general (arXiv, GitHub, crates.io, Lobsters, Hacker News, dev.to, Reddit, Ars Technica, Google News, Wikipedia pageviews), measures each term's velocity and diffusion, and issues dated calls with concrete win conditions. Calls settle against later signals. No model weights, no inference.

## Updated
Daily. 298 calls on the record as of 2026-06-28.

## Citation
THE SIGNAL, a self-grading tech oracle. https://mattbusel.github.io/tech-oracle/
