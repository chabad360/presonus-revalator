# Digital audio meter and mixer-fader scale research

## Conclusion

- **Digital peak meter:** equal dB spacing is not the only established convention. The publicly available metadata/preview for **IEC TR 60268-18:1995** confirms that it is specifically about digital audio peak-level meters and contains a display-scale clause plus an annex on scale marking, but the public preview stops before those clauses. The exact IEC table could therefore not be checked directly without buying the report. [IEC product page and preview](https://webstore.iec.ch/en/publication/1217)
- A high-quality implementation explicitly calling itself IEC 60-268-18 compliant is OBS. It uses a **piecewise-linear-in-dB, nonlinear-in-position** scale with anchors `−∞→0%`, `−60→2.5%`, `−50→7.5%`, `−40→15%`, `−30→30%`, `−20→50%`, `−9→75%`, `0→100%`. [OBS declaration/table](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.h#L33-L64), [OBS formula](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.c#L101-L157)
- Ardour independently confirms the **same general nonlinear breakpoint family below −20 dB**, but uses `−20→50%`, `−10→75%`, `0→100%`, and extends to `+6→115%`. Thus it confirms the nonlinear convention but not OBS's exact `−9 dB` upper breakpoint. [Ardour formula](https://github.com/Ardour/ardour/blob/e0d72575b1ebf9609345a02ca972446ce3fd1446/libs/ardour/ardour/logmeter.h#L47-L100)
- **Mixer fader:** no IEC/EBU standard specifying a universal `−96..+10 dB` travel curve or tick set was found. IEC TR 60268-18 is a *meter* report, not a mixer-fader standard. Primary implementation sources expose several incompatible choices, so a fader curve must be treated as an application/hardware policy rather than “the standard.” OBS offers cubic, IEC-derived, and logarithmic faders side by side; JUCE exposes configurable skew or arbitrary mapping functions; LMMS uses its own cubic normalized-dB curve. [OBS alternatives](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.h#L33-L68), [JUCE `NormalisableRange`](https://github.com/juce-framework/JUCE/blob/7aae7d8e8deb8413bb01633d2795ef9974a181c5/modules/juce_core/maths/juce_NormalisableRange.h), [LMMS mapping](https://github.com/LMMS/lmms/blob/4e677cb6c6ab7ac8bec221097053753c1eb273e3/src/gui/widgets/Fader.cpp#L63-L65)

## IEC-style digital peak mapping

Let `q(d)` be nominal deflection, with `q(0)=1`, for dB value `d`. The OBS implementation gives:

```text
−60 ≤ d < −50: q = 0.025 + 0.005   (d + 60)
−50 ≤ d < −40: q = 0.075 + 0.0075  (d + 50)
−40 ≤ d < −30: q = 0.150 + 0.015   (d + 40)
−30 ≤ d < −20: q = 0.300 + 0.020   (d + 30)
−20 ≤ d <  −9: q = 0.500 + (0.25/11)(d + 20)
 −9 ≤ d:       q = 0.750 + (0.25/9) (d + 9)
```

Source: [OBS implementation](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.c#L130-L157).

For a track whose top is `+6 dB`, extend the final segment and normalize with `p(d)=q(d)/q(+6)=q(d)/(7/6)`. The key positions, measured upward from the bottom of the full IEC-style display, are:

| dB | `p` for top `+6 dB` |
|---:|---:|
| −60 | 2.14% |
| −50 | 6.43% |
| −40 | 12.86% |
| −30 | 25.71% |
| −20 | 42.86% |
| −9 | 64.29% |
| 0 | 85.71% |
| +6 | 100% |

This `+6` normalization is a derivation from the cited OBS formula, not wording directly verified in IEC. Ardour's independently implemented `+6 dBFS` version is very close but uses `−10→approximately 65.22%`, `0→86.96%`, `+6→100%`; its source calls `+6→115%` an arbitrary endpoint. [Ardour formula and qualification](https://github.com/Ardour/ardour/blob/e0d72575b1ebf9609345a02ca972446ce3fd1446/libs/ardour/ardour/logmeter.h#L47-L75)

## EBU material

- Historical EBU Tech 3205-E (type IIb analogue/quasi-peak PPM) explicitly requires **twelve approximately equal 2 dB divisions**, with principal marks every 4 dB around a mid-scale `Test` mark and an extra `+9 dB` mark. This is uniform dB spacing, but it is an analogue PPM convention and the document now says it is superseded by R 128. [EBU Tech 3205-E, §1.1, p. 7](https://tech.ebu.ch/docs/tech/tech3205.pdf)
- Current EBU Tech 3341 defines EBU loudness-meter ranges (`−18..+9 LU` and `−36..+18 LU`) but says physical properties such as size, colours, and design are not part of the document. It requires display of maximum true peak but gives no physical dBTP peak-meter tick-position mapping. [EBU Tech 3341, §§2.7–2.8, pp. 6–7](https://tech.ebu.ch/docs/tech/tech3341.pdf)
- Consequently, no current EBU source was found that standardizes the requested `−60..+6 dBFS` physical peak-meter ticks.

## Source-backed tick sets

There is no single universal label set for either requested range.

### `−60..+6 dB` peak meter

Two defensible source-backed choices are:

1. **IEC-style anchor labels:** `−60, −50, −40, −30, −20, −9, 0, +6`, positioned by the piecewise mapping above. The `+6` mark is the chosen range endpoint; the other anchors come from the OBS IEC table. [OBS table](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.h#L45-L62)
2. **Ardour's +6 dBFS digital peak labels:** `−50, −40, −30, −24/−25, −20, −18, −15, −10, −5, −3, 0, +3`, with additional minor ticks including `−60` and `+1..+5`. This is an implementation convention, not an IEC/EBU requirement. [Ardour major labels](https://github.com/Ardour/ardour/blob/e0d72575b1ebf9609345a02ca972446ce3fd1446/gtk2_ardour/meter_patterns.cc#L753-L778), [Ardour minor ticks](https://github.com/Ardour/ardour/blob/e0d72575b1ebf9609345a02ca972446ce3fd1446/gtk2_ardour/meter_patterns.cc#L540-L577)

### `−96..+10 dB` mixer fader

No standardized tick list was found. Source-backed examples differ:

- LMMS draws fader tick marks every `6 dB`, centered on `0 dB`, while using a cubic normalized-dB position curve. For a `+10 dB` maximum that rule starts at `+6`, then `0, −6, −12, …, −96`; adding `+10` and `−96/−∞` endpoint labels would be an application choice. [LMMS tick loop](https://github.com/LMMS/lmms/blob/4e677cb6c6ab7ac8bec221097053753c1eb273e3/src/gui/widgets/Fader.cpp#L717-L748), [LMMS curve](https://github.com/LMMS/lmms/blob/4e677cb6c6ab7ac8bec221097053753c1eb273e3/src/gui/widgets/Fader.cpp#L446-L453)
- OBS's IEC-derived fader anchors are `−∞, −60, −50, −40, −30, −20, −9, 0`; positive-gain marks are not standardized by that table. [OBS IEC fader table](https://github.com/obsproject/obs-studio/blob/a2400fe9803811b6e691a019e8bc2b1acfaf6250/libobs/obs-audio-controls.h#L45-L64)
- A Yamaha CL digital console documents actual fader range as `+10 dB` to `−138 dB/−∞`, but its owner manual does not publish a mathematical travel mapping or machine-readable tick set. [Yamaha CL Owner's Manual, specifications p. 58](https://download.yamaha.com/files/tcm:39-798479)

## Difference from this project (`egui-audio-controls` 0.1.0, egui 0.36.1)

- The installed meter uses **uniform dB spacing**: `p=(d−L)/(H−L)`, so for `−60..+6`, `p=(d+60)/66`. It paints ticks `−60, −48, −36, −24, −18, −12, −6, −3, 0, +6`. See `src/audio-controls/src/meter.rs:143`, `src/audio-controls/src/meter.rs:232`, and `src/audio-controls/src/meter.rs:244`. This differs substantially from the IEC-style nonlinear display; for example, `−20 dB` is at 60.61% in the project but about 42.86% using the OBS IEC-style mapping normalized to `+6`.
- The installed fader uses the custom curve `p=((d−L)/(H−L))³`; inverse `d=L+(H−L)∛p`. For `−96..+10`, unity is at `p=(96/106)³=74.28%`. See `src/audio-controls/src/scale.rs:3`, `src/audio-controls/src/scale.rs:39`, and `src/audio-controls/src/scale.rs:56`. This is not the OBS cubic fader (`linear gain=p³`) and is not established by an IEC/EBU fader standard. It is closest in form to LMMS's cubic normalized-dB mapping, though LMMS uses a `−120 dB` floor. [LMMS curve](https://github.com/LMMS/lmms/blob/4e677cb6c6ab7ac8bec221097053753c1eb273e3/src/gui/widgets/Fader.cpp#L446-L453)
- The installed fader's visible `−96..+10` labels are `−∞` at the `−96` endpoint, then `−60, −48, −36, −24, −18, −12, −6, −3, 0, +6`, and `+10` at the top. See `src/audio-controls/src/fader.rs:218` and `src/audio-controls/src/fader.rs:225`. This is a project-specific sparse set; no primary standard supporting exactly that set was found.

## Recommendation

If the goal is a standards-recognizable **digital peak display**, use the IEC-style piecewise scale and anchor ticks, while documenting whether the upper segment follows the OBS `−9 dB` breakpoint or Ardour's `−10 dB` approximation/variant. If the goal is merely a readable dBFS meter, uniform dB spacing is valid UI policy but should not be called IEC 60268-18.

For the **mixer fader**, retain or revise the current cubic curve based on desired ergonomics or matching the physical PreSonus UI; do not describe it as standardized. No authoritative source was found for a universal `−96..+10 dB` curve or exact tick set.
