# Track A — CVF2 frame encoding

Keep the public `Frames` API (`count`, `width`, `delay_ms`, `draw`) unchanged.
Keep kernel changes in `src/frames.rs`. Track B owns `src/main.rs` and
`src/console.rs`. Document any API change as a proposal for a follow-up
integration commit. Verify CVF1 compatibility and rerun the QEMU pointer proof.
