# iPad Air 1 Wi-Fi (A1474 / J71 / iPad4,1 / A7 S5L8960X): bare-metal research note

Researched 2026-11-24 for Corvus OS. Desk research only; no hardware was touched.
Labels: **[C]** = confirmed in a primary source (code, DTS, project docs). **[S]** = speculation or inference.

## Verdict

- **Booting custom code is feasible, tethered, and runs from RAM only.** A7 has the checkm8 bootrom bug, which Apple can't patch. pongoOS is loaded over USB from DFU. pongoOS can then jump to a raw binary you upload (`bootr`). [C]
- **Display: yes.** iBoot sets up the panel and leaves a linear 32-bit framebuffer. Its address is passed in `boot_args->Video`. pongoOS already draws to it. On the sister board J72 it is measured at 1536x2048 portrait, stride 6144. [C for J72, S for J71]
- **Touch is the hard part.** The controller is Apple/Broadcom "N1"/Z2 multitouch on SPI2. It needs firmware built from iOS `J71.mtprops` plus per-unit calibration. Open drivers exist only out of tree (Orchard's `apple_z2` J72 variant), and nobody has run one on J71. [C]
- **Orchard really does list J71 as `experimental`.** In Orchard that only means "Orchard builds for it". The file itself says nothing has run on this board, and every feature is `untested`/`missing`. All the evidence comes from the J72 (Cellular). [C]
- **Watch the name: Orchard's `J71b` is the iPad 6 (iPad7,5), not your iPad Air.** "J71t/J71s" are iPad 5 boards. Don't mix up their docs. [C]

## Capability table

| Capability | Status for J71 | Confidence | Source |
|---|---|---|---|
| checkm8 bootrom exploit on A7 | ipwndfu lists s5l8960x as supported. The README calls it "not reliable, repeat" and warns of possible bricking | C | ipwndfu README |
| checkra1n on A7 | Supported on macOS. checkra.in says "A7 devices will currently not work with the Linux version" | C | checkra.in |
| pongoOS on A7 | A7 code paths (`socnum == 0x8960`) are in tz/aes/xnu drivers. HoolockLinux recommends the pongoOS route (via palera1n) for A7 on a Linux host | C | pongoOS src, HoolockLinux SETUP.md |
| Raw payload boot | `/send file` then `bootr`: x0 = iBoot `boot_args*`, x8 = original entry. The image runs from the upload buffer, at a physical address you don't choose, so the payload must be position-independent or relocate itself | C | pongoOS `src/kernel/entry.c`, `src/boot/jump_to_image.S` |
| Mainline Linux DT for J71 | `s5l8960x-j71.dts` ("Apple iPad Air (Wi-Fi), J71, iPad4,1 (A1474)"), merged in 6.13 | C | kernel tree, LKML patch series |
| HoolockLinux A7 features | UART, AIC, GPIO, WDT, PMGR, SMP, I2C, cpufreq: upstream. Main display (boot FB) 6.13. USB2 device mode, RTC: out of tree. Display pipe, GPU: TBA. Storage: WIP | C | HoolockLinux `features/A7.md` |
| Orchard J71 | `status = "experimental"`, boot method `checkm8-pongo` "not run on this board with Orchard yet". display/touch/wifi/usb_serial/storage all `untested` | C | `OrchardLinux/orchard devices/j71/device.toml`, `docs/compatibility.md` |
| Orchard J72 (closest proven sibling) | Display, backlight, touch, buttons, internal storage, USB serial, USB network: working (thisiscamk, Nov 2026) | C | `devices/j72/device.toml`, thisiscamk/ipad-air-j72-linux |
| Framebuffer handoff | iBoot `Boot_Video {v_baseAddr, v_display, v_rowBytes, v_width, v_height, v_depth}`. pongoOS maps it, treats it as 32-bit pixels, and writes a simplefb DT node with `format="a8b8g8r8"` | C | pongoOS `pongo.h`, `fb.c`, `modules/linux/linux.c` |
| FB geometry | J72: phys `0x83d200000`, 1536x2048, stride `0x1800` (6144), 32 bpp. J71 likely identical (same panel and SoC), but not measured | C (J72) / S (J71) | thisiscamk `code/display/cascadia/README.md` |
| Exact pixel channel order | pongoOS hard-codes `a8b8g8r8` for Linux. Check `v_depth` and test with a colour bar | S | pongoOS |
| Touch controller | "N1 on SPI2 as on the J72 (ADT spi2/multi-touch)". Firmware `j72-fwstart.z2fw` is built from `J71.mtprops`. The J72 notes mention a BCM5976-related part from teardowns | C (ADT) / S (BCM part) | Orchard `devices/j71/*`, J72-PROGRESS |
| Open touch driver | Mainline `apple_z2` is a different variant (Mac Touch Bar). Orchard carries a J72 variant that works on J72 and is untested on J71. It needs your own unit's calibration | C | Orchard j72/j71 device files, J72 QUICKSTART |
| USB console | pongoOS has a Synopsys OTG USB driver and talks to host `pongoterm`. Linux uses dwc2 gadget for CDC-ACM `/dev/ttyACM0` + NCM. For a Rust kernel you'd write your own DWC2 device stack | C (pongo/Linux) / S (effort) | pongoOS `drivers/usb/synopsys_otg.c`, HoolockLinux SETUP.md |
| Hardware UART | Upstream Samsung-style UART driver plus DT exist. pongoOS passes `earlycon=hx_uart`. Physical access on an iPad (Lightning debug cable / board pads) isn't documented for J71 | C (driver) / S (access) | HoolockLinux A7.md, pongoOS linux.c |
| Wi-Fi (later) | Orchard says BCM43241 on HSIC (firmware `4324b5/syrah.trx`). TheAppleWiki says BCM43342. The sources disagree | conflicting | Orchard j71 firmware.json, TheAppleWiki |

## Risks

- **RAM boot alone is low risk.** checkm8, pongoOS and your payload run from RAM. A forced reboot returns to stock iPadOS, and nothing is written to NAND. Orchard and the J72 port require "a normal return to iOS" as the acceptance check. [C]
- **Tethered only.** Every boot needs a host and DFU entry. There is no persistent untethered path on A7. [C]
- **Exploit flakiness.** On A7, checkm8 often takes several attempts (ipwndfu: "not reliable"). The checkra1n Linux build doesn't support A7, so use macOS checkra1n or palera1n's pongo mode. [C]
- **Writing to NAND/NOR is the real brick risk.** Examples: an Orchard install that resizes APFS, or ipwndfu NOR flashing. Recovery is a DFU restore to iPadOS 12.5.8 (16H88, the IPSW Orchard pins). Don't do this. [C/S]
- **Drivers can do physical harm.** Bad PMIC, charger or backlight writes can damage hardware. Orchard gates charging and speakers behind safety logic for this reason. Leave PMU/D2089 and charging alone in your own kernel. [S, based on Orchard's safety notes]
- **Unit-specific data.** Touch calibration and Wi-Fi NVRAM/MAC come from your own device. Never copy them from another unit. [C]

## Recommended next safe experiment (research/tethered, read-only)

1. Get a Mac, or Linux with palera1n, plus pongoterm and a known-good Lightning data cable. Confirm the iPad boots iOS normally first.
2. Enter DFU and boot pongoOS (`palera1n -lp -k Pongo.bin` or checkra1n `-p`), retrying as needed. Then run **`bootargs`** in pongoterm. It is read-only and prints `boot_args`, which records the real J71 `v_baseAddr/rowBytes/width/height/depth`.
3. Build a tiny position-independent AArch64 Rust payload (no_std). It should read `x0 -> boot_args.Video`, paint RGB bars plus a "Corvus" marker, and then `wfe`-loop. It must not touch PMGR, PMU, NAND or USB.
4. Run `/send corvus.bin` and then `bootr`. Check the colours to confirm the channel order, then hold Power+Home to go back to iOS.
5. Afterwards, chain through m1n1 (HoolockLinux `idevice` branch) to get its proxy, ADT dump and the hardware tracing workflow. Add a DWC2 CDC-ACM console only after the framebuffer works.

## URLs

- Orchard repo: https://github.com/OrchardLinux/orchard (J71 file: `devices/j71/device.toml`; matrix: `docs/compatibility.md`; checked at commit eab974c, 2026-11-24)
- J72 port (thisiscamk): https://github.com/thisiscamk/ipad-air-j72-linux
- HoolockLinux docs: https://github.com/HoolockLinux/docs (`features/A7.md`, `tutorials/SETUP*.md`)
- HoolockLinux org (linux, m1n1 forks): https://github.com/HoolockLinux
- pongoOS: https://github.com/checkra1n/pongoOS
- checkra1n: https://checkra.in/
- palera1n: https://github.com/palera1n/palera1n
- ipwndfu: https://github.com/axi0mX/ipwndfu
- J71 DTS: https://gbmc.googlesource.com/linux/+/9dba9a45c348e8460da97c450cddf70b2056deb3/arch/arm64/boot/dts/apple/s5l8960x-j71.dts
- A7 common DTSI (simplefb filled by loader): https://kernelsources.org/source/xref/linux/arch/arm64/boot/dts/apple/s5l8960x-common.dtsi
- A7 DT patch series (Nick Chan, 2024): https://lists.infradead.org/pipermail/linux-arm-kernel/2024-September/960833.html
- Konrad Dybcio A7/A8 mainline boot (2022): https://hackaday.com/2022/06/12/boot-mainline-linux-on-apple-a7-a8-and-a8x-devices
- TheAppleWiki iPad Air: https://theapplewiki.com/wiki/J71AP
- Project Sandcastle (Corellium, A10 only, historical): https://github.com/corellium/projectsandcastle
