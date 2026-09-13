# Platform Migration Research: Unitree GO2, ROS 2 Distros, and Rust

**Prepared:** 2026-09-13
**Scope:** primary-source investigation for (a) whether the GO2 constrains the ROS 2 distro, (b) whether a Rust migration is viable, and (c) how to package and deploy ROS 2 workloads to edge robots.
**Method:** official docs, upstream GitHub repos/READMEs/CHANGELOGs, `crates.io`/PyPI metadata APIs, REP-2000. Every non-obvious claim is cited with a URL. Claims that could not be tied to a retrievable primary source are explicitly marked **[unverified]**.

> **Version snapshot used throughout.** Where a number is load-bearing, the source date is given. The live `endoflife.date` page used for ROS EOL data was last updated 2026-07-26; `rclrs 0.7.0` was published 2026-01-18; `unitree_ros2` `version.txt` = `0.3.0`.

---

## TL;DR (full reasoning in the Bottom Line section)

1. **The GO2 is not tied to Foxy.** Unitree's own `unitree_ros2` README lists Ubuntu 20.04/Foxy and Ubuntu 22.04/Humble (recommended), and the robot link is at the **DDS level (CycloneDDS 0.10.2)**, not at the ROS-distro level. Foxy is EOL since 2023.
2. **A full Rust rewrite is not ready as a single step**, but a **mixed C++/Rust/Python architecture is viable**, and a **Rust drop-in that speaks DDS directly to Unitree topics is technically viable today** (cyclonedds-rust / rustdds / dust_dds). ROS-native Rust (`rclrs`) supports Humble→Rolling, **not Foxy**.
3. **Recommended deployment model:** target **ROS 2 Jazzy on Ubuntu 24.04** for the long-lived LTS, build **signed `.deb`s with `bloom`**, host a **signed apt repo (aptly/reprepro) with version pinning**, run nodes as **systemd services with journald + systemd credentials**, and use **A/B image updates (RAUC or Mender)** rather than in-place apt upgrades on the robot.

---

## A. Unitree GO2 compatibility and constraints

### A1. What must actually match between the control PC and the GO2?

The coupling is **CycloneDDS + the Unitree message/IDL layer**, *not* a specific ROS 2 distribution.

- `unitree_sdk2` (the C++ SDK) is described as "an easy-to-use robot communication mechanism based on Cyclonedds" and `unitree_ros2` states the underlying layers of GO2/B2/H1 are "compatible with ROS2" because ROS 2 also uses DDS — ROS 2 messages can be used directly for control without wrapping the SDK. → <https://github.com/unitreerobotics/unitree_ros2> (Introduction), <https://github.com/unitreerobotics/unitree_sdk2> (README).
- `unitree_ros2`'s "System requirements" table lists two tested combinations: **Ubuntu 20.04 + Foxy**, and **Ubuntu 22.04 + Humble (recommend)**. → <https://github.com/unitreerobotics/unitree_ros2>
- The DDS version that must be matched is **CycloneDDS 0.10.2**. `unitree_ros2` says "The cyclonedds version of Unitree robot is 0.10.2", and `unitree_sdk2_python` pins `cyclonedds==0.10.2`. → <https://github.com/unitreerobotics/unitree_ros2>, <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/setup.py>
- Practical implication: what must line up is the **RMW implementation (`rmw_cyclonedds_cpp`) + CycloneDDS 0.10.x + the exact topic/message definitions + domain id + network interface**. The ROS distro version only affects which `rmw_cyclonedds_cpp`/`rosidl` packages you install, which is exactly why both Foxy and Humble work.

**Verdict:** No official requirement that the control PC run Foxy. "The GO2 environment is Foxy" is an **obsolete setup assumption**, not a constraint.

### A2. Official Unitree SDKs — supported distros/Ubuntu

| SDK | Type | OS / distro support (official) | Evidence |
|---|---|---|---|
| `unitreerobotics/unitree_ros2` | ROS 2 bridge / msg packages for GO2, B2, H1, G1 | **Ubuntu 20.04 + Foxy**, **Ubuntu 22.04 + Humble (recommended)**; README tells you to substitute the distro name for others | <https://github.com/unitreerobotics/unitree_ros2> |
| `unitreerobotics/unitree_sdk2` | C++ SDK (DDS) | **Ubuntu 20.04 LTS**, CPU **aarch64 and x86_64**, GCC 9.4.0 | <https://github.com/unitreerobotics/unitree_sdk2> |
| `unitreerobotics/unitree_sdk2_python` | Python SDK (DDS) | Python ≥ 3.8; `cyclonedds==0.10.2`, numpy, opencv-python; no OS-specific requirement stated | <https://github.com/unitreerobotics/unitree_sdk2_python>, <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/setup.py> |

**Branch layout:** `unitree_ros2` does **not** have per-distro branches for Foxy/Humble. Its only branches are `master` and `ros2_service_support`. → <https://api.github.com/repos/unitreerobotics/unitree_ros2/branches>. The single codebase is distro-agnostic; you change which ROS environment you source.

**CI evidence:** the repo's workflows are `build-foxy.yml` and `build-humble.yml` (plus `auto-tag.yml`, `release.yml`) — i.e. Foxy and Humble are the only CI-tested distros upstream. → <https://github.com/unitreerobotics/unitree_ros2/tree/master/.github/workflows>. Humble CI was fixed by merged PR #20. → <https://github.com/unitreerobotics/unitree_ros2/pull/20>

**Version:** `unitree_ros2` `version.txt` = `0.3.0`; v0.3.0 (2025-08-15) explicitly notes a Foxy fix and that a "Humble feature marked in v0.2.0 can now be used in Foxy as well", showing the two distros are maintained in parallel. → <https://github.com/unitreerobotics/unitree_ros2/blob/master/CHANGELOG.md>, <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/version.txt>

### A3. Does `unitree_sdk2_python` depend on rclpy or ROS at all?

**No.** Its declared runtime dependencies are only `cyclonedds==0.10.2`, `numpy`, and `opencv-python`:

```
install_requires=[
      "cyclonedds==0.10.2",
      "numpy",
      "opencv-python",
],
```
→ <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/setup.py>

The core channel implementation imports only `cyclonedds.*` (Domain, DomainParticipant, DataWriter, DataReader, Topic, Qos, Listener), never `rclpy`. → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/unitree_sdk2py/core/channel.py>

`pyproject.toml` declares only `setuptools`/`wheel` as build requirements (a `setup.py`-style package; not a modern PEP 621 metadata file). → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/pyproject.toml>

**Implication:** any language binding to the same DDS/IDL layer (Rust included) can replace the Python SDK without a ROS runtime.

### A4. DDS implementation, domain id, network interface, topics

**DDS implementation:** Eclipse **CycloneDDS 0.10.2** (see A2/A3). `unitree_ros2` additionally sets `RMW_IMPLEMENTATION=rmw_cyclonedds_cpp` in its `setup.sh`. → <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/setup.sh>

**Domain id:** the SDKs default to **domain 0**:
- Python: `ChannelFactoryInitialize(id: int = 0, networkInterface=None)`; every GO2 example calls `ChannelFactoryInitialize(0)`. → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/unitree_sdk2py/core/channel.py>, <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/example/go2/low_level/go2_stand_example.py>
- C++: `ChannelFactory::Init(int32_t domainId, const std::string& networkInterface = "")`. → <https://github.com/unitreerobotics/unitree_sdk2/blob/main/include/unitree/robot/channel/channel_factory.hpp>
- Note the config template uses `<Domain Id="any">`, so the participant id passed to `Init` is authoritative. → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/unitree_sdk2py/core/channel_config.py>
- Community GO2 bring-up unsets `ROS_DOMAIN_ID` (defaults to 0). → <https://github.com/jkk-research/JKK_UNITREE_GO2>

**Network interface / addresses:**
- Control PC is put on the robot's `192.168.123.0/24` network, e.g. static `192.168.123.99/255.255.255.0`, and the robot interface name is passed to CycloneDDS. → <https://github.com/unitreerobotics/unitree_ros2> ("Network configuration")
- CycloneDDS is pinned to that interface via `CYCLONEDDS_URI` with an explicit `<NetworkInterface name="...">`. → <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/setup.sh>
- Community GO2 docs give the robot's onboard address as **192.168.123.18** (SSH), the robot/LiDAR peer as **192.168.123.161**, and show a CycloneDDS `<Discovery><Peers><Peer address="192.168.123.161"/>` config. → <https://github.com/jkk-research/JKK_UNITREE_GO2>

**Topics / services:**
- Low-level: `rt/lowcmd` (LowCmd, publish) and `rt/lowstate` (LowState, subscribe). → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/example/go2/low_level/go2_stand_example.py>
- Sport-mode state: `rt/sportmodestate` / `lf/sportmodestate`; sport control via request/response (`/api/sport/request` in ROS; `rt/api/<service>/request` in the SDK). → <https://github.com/unitreerobotics/unitree_ros2> ("State acquisition", "Robot control"), <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/unitree_sdk2py/core/channel_name.py>
- Unitree's API topic namespace is `rt/api/<serviceName>/{request,response}`. → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/unitree_sdk2py/core/channel_name.py>
- Under `unitree_ros2` the topics appear as ROS topics (e.g. `sportmodestate`, `lowstate`, `/lowcmd`, `/api/sport/request`). → <https://github.com/unitreerobotics/unitree_ros2>

**Note for any non-ROS client (incl. Rust):** to join these topics you must not only match the topic name and domain but also the DDS **type** (name + XTypes type information / serialized layout) that Unitree publishes. This is a real integration cost and the main risk in B2; see the caveat there.

### A5. Does the GO2 have an onboard computer that runs ROS? What OS?

**Yes, GO2 EDU/Research units ship with an onboard NVIDIA Jetson, and it can run ROS, but Unitree directs developers to an external PC.** The evidence is mixed, so it is split here:

- **Onboard hardware exists.** Third-party GO2 integration docs state a ROS 2 workspace "should already be available and built on GO2's **Nvidia board**", and their install path is "Onboard PC Installation" vs "External PC Installation". → <https://docs.quadruped.de/projects/go2/html/go2_ros2_installation.html>
- **Onboard OS (community, detailed).** A GO2 EDU/NX flashing/setup guide describes the onboard **Jetson Orin Nano/NX**, default **JetPack 5.1.1 (Ubuntu 20.04)**, with an upgrade path to **JetPack 6.x (Ubuntu 22.04+)**; it warns the QSPI prevents downgrade; SSH `unitree@192.168.123.18` (password `123`). It also states the onboard stack builds on **ROS 2 Humble**, Unitree SDK2, Unitree ROS2, and the `go2_ros2_sdk` community driver. → <https://github.com/jkk-research/JKK_UNITREE_GO2>
- **Unitree's official guidance points at an external PC.** Unitree's developer docs (JS-rendered; content only retrievable via a search-engine snippet) reportedly say: *"It is recommended to develop on Ubuntu 20.04 or Ubuntu 22.04. Development on Mac or Windows systems is currently not supported, nor is development on Go2's internal computer."* — **Quote is from a DuckDuckGo snippet of `support.unitree.com` developer docs; the exact page URL could not be retrieved because the site is client-rendered. Treat as [unverified] for exact wording/URL, but it is consistent with Unitree's SDK README guidance (external PC over Ethernet).** → doc index: <https://support.unitree.com/home/en/developer/Quick_start>
- The SDK prebuilt environment lists **aarch64** support, which is consistent with ARM/Jetson builds even though official *development* guidance is external. → <https://github.com/unitreerobotics/unitree_sdk2>

**Verdict:** the onboard Jetson exists and community users run Humble there, but for this project's control stack, treat the **external control PC** as the supported deployment target unless you deliberately adopt the onboard Jetson (which then constrains you to JetPack/Ubuntu versions).

### A6. Documented path to Humble/Jazzy with GO2?

- **Humble:** officially tested and *recommended* by Unitree. → <https://github.com/unitreerobotics/unitree_ros2>. Multiple merged PRs fix Humble builds (PR #20 CI, and the Humble API additions in `v0.2.0`). → <https://github.com/unitreerobotics/unitree_ros2/blob/master/CHANGELOG.md>, <https://github.com/unitreerobotics/unitree_ros2/pull/20>
- **Jazzy:** Community-supported, not yet in Unitree's supported table. There is an **open, unmerged** PR titled "Adds 24.04/jazzy as a supported combination" (opened 2024-07-19, still open as of its last update 2025-10-17). → <https://github.com/unitreerobotics/unitree_ros2/pull/3>. There is no `build-jazzy.yml` workflow. → <https://github.com/unitreerobotics/unitree_ros2/tree/master/.github/workflows>
- Independent community GO2 stack confirming Humble onboard is `go2_ros2_sdk`. → referenced from <https://github.com/jkk-research/JKK_UNITREE_GO2>

**Verdict:** Humble is an officially supported, low-risk move. Jazzy is viable via the DDS layer but is **not officially validated by Unitree as of the latest repo state** — budget for a bring-up/validation phase.

---

## B. Rust + ROS + DDS options

### B1. `ros2-rust/ros2_rust` (`rclrs`)

**Version / release cadence** (crates.io API):
- Latest: **`rclrs 0.7.0`, published 2026-01-18**, MSRV Rust **1.85**, Apache-2.0.
- Prior: `0.6.0` (2025-10-27), `0.5.1` (2025-08-23), `0.5.0` (2025-08-20), `0.4.1` (2023-11-28).
- Total downloads ~55k; ~17k in the recent window.
→ <https://crates.io/api/v1/crates/rclrs>

**Supported ROS 2 distros:** the README has explicit sections for **Humble**, **Jazzy**, **Kilted**, **Lyrical Luth**, and **Rolling**; the repo ships `.repos` files `ros2_rust_{humble,jazzy,kilted,lyrical,rolling}.repos`. **Foxy is not listed and no `ros2_rust_foxy.repos` exists.** → <https://github.com/ros2-rust/ros2_rust>

**Features present:** message generation for all ROS message types, pub/sub (incl. async), loaned/zero-copy messages, dynamic messages, tunable QoS, clients/services (incl. async), actions (async), timers, parameters + services, logging/rosout, graph queries, guard conditions/wait sets, clock/time, worker + executor patterns. → <https://github.com/ros2-rust/ros2_rust>

**Known limitations / maturity:**
- README: "Since the client library is still rapidly evolving, there are **no stability guarantees** for the moment." Missing items are tracked in the issue list. → <https://github.com/ros2-rust/ros2_rust>
- Installation is **not turnkey**: it requires `colcon-cargo` + `colcon-ros-cargo`, plus workaround packages (`example-interfaces`, `test-msgs`) due to issue #557, and (as of 2025-01-21) cloning `rosidl_rust` for the code generator. → <https://github.com/ros2-rust/ros2_rust>
- Foxy cannot be targeted: it is EOL, and `rclrs 0.7.0` alone requires Rust 1.85.

**Verdict:** `rclrs` is a credible option for **Humble or newer**, but it is pre-1.0 with no API stability and a build-tooling tax. It is **not** a path for Foxy.

### B2. Rust DDS crates usable *without* `rclrs`

All three implement DDS/RTPS at the wire level, which is the same interop layer Unitree uses. The `unitree_sdk2_python` proof point (pure DDS, no ROS) shows you do not need ROS to talk to these topics.

| Crate | Latest / date | Maintenance | IDL / codegen | Interop notes |
|---|---|---|---|---|
| **`eclipse-cyclonedds`** (`cyclonedds-rust`) | `0.0.4` (2026-06-24) | Official Eclipse CycloneDDS Rust binding; crate first published 2026-06-15 (very new) | Derive macro `#[derive(Topicable)]` + serde; no `.idl` code generator described | **Same C library as Unitree's DDS** — it wraps CycloneDDS C (`vendored` feature compiles/links a bundled version). Strongest interop story, but **0.0.x, tiny adoption (~150 downloads)**. → <https://github.com/eclipse-cyclonedds/cyclonedds-rust>, <https://crates.io/api/v1/crates/eclipse-cyclonedds> |
| **`rustdds`** | `0.14.2` (2026-09-01) | Mature-ish; 2,053 commits; maintained by Atostek; ~390k downloads; for ROS use `ros2-client`. → <https://github.com/Atostek/RustDDS> | Pure Rust; uses **serde** generics instead of IDL codegen ("We do not rely on code generation") | README: "implementation is complete enough to do data exchange with ROS2"; has published interop reports and fixes explicitly targeting CycloneDDS/FastDDS. → <https://github.com/Atostek/RustDDS> |
| **`dust_dds`** | `0.16.0` (2026-09-11) | Actively developed by S2E Systems; ~115k downloads; 0.15.0 (Mar 2026) reached feature-complete DCPS+RTPS default features. → <https://crates.io/api/v1/crates/dust_dds> | Pure Rust; repository `s2e-systems/dust-dds`; XTypes support (`xtypes-xml` feature). Homepage: <https://s2e-systems.com/products/dust-dds> | Pure-Rust RTPS; interop with other vendors is an explicit goal but must be verified per-topic. → <https://crates.io/api/v1/crates/dust_dds> |
| **`ros2-client`** | `0.10.1` (2026-08-10) | Built on RustDDS; feature flags default to `jazzy`, with `humble/iron/kilted/lyrical` (additive). MSRV Rust 1.88. ~260k downloads. → <https://crates.io/api/v1/crates/ros2-client> | Ships an `msggen` binary for message generation | **No `rclrs`/ROS runtime needed**; talks ROS 2 topics directly. This is the closest "ROS-in-Rust without ROS" option. Feature list confirms Humble is supported (`humble` feature), **Foxy is not**. → <https://crates.io/api/v1/crates/ros2-client> |

**Can they interoperate with CycloneDDS-based Unitree DDS?** In principle yes, because DDSI-RTPS is the standardized wire protocol and cross-vendor interop is a core DDS goal. The ROS 2 docs caution that cross-vendor DDS communication "is not guaranteed under all circumstances" and recommend using the same RMW across a system. → <https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/client-libraries/About-Different-Middleware-Vendors.rst>. The safest Rust choice for GO2 is therefore **`eclipse-cyclonedds` (same underlying C implementation)** or **`rustdds`/`ros2-client` (published CycloneDDS interop work)**.

**Caveat / unverified:** none of the Rust DDS crates has a published, first-party example of talking to Unitree GO2's exact IDL types and XTypes type names. Type-name/type-hash matching is the likely failure mode. **Treat "does it actually match GO2 topics end-to-end" as requiring a spike, not as a proven fact.**

### B3. Rust crates for the other concerns

crates.io metadata pulled 2026-09-13:

| Need | Crate | Latest / date | Maintenance read | Covers the need? |
|---|---|---|---|---|
| AprilTag detection | `apriltag` | `0.4.0` (2023-01-28) | High usage (4.4M downloads) but **no release since Jan 2023**; wraps the C AprilTag lib. → <https://crates.io/api/v1/crates/apriltag> | Probably works; **risk: stale**. Alternative is `opencv` ArUco/AprilTag. |
| CV | `opencv` | `0.100.1` (2026-07-31) | Very active (921k recent downloads). → <https://crates.io/api/v1/crates/opencv> | Yes — full OpenCV bindings. |
| Camera capture | `nokhwa` | `0.10.11` (2026-05-15) | Active, cross-platform. → <https://crates.io/api/v1/crates/nokhwa> | Yes. |
| Camera capture (Linux) | `v4l` | `0.14.0` (2023-05-12) | Very high downloads but **no release since May 2023**. → <https://crates.io/api/v1/crates/v4l> | Works for V4L2; stale. `nokhwa` preferred for new code. |
| MuJoCo | `mujoco-rs` | `6.0.1+mj-3.12.0` (2026-09-07) | Very active (davidhozic); tracks MuJoCo 3.12; native Rust viewer. → <https://crates.io/api/v1/crates/mujoco-rs> | Yes. Note this is a **different crate lineage** from the older `TheButlah/mujoco-rs`. |
| MuJoCo (FFI, old) | `mujoco-sys` | `0.0.1` (2020-08-12) | **Abandoned** (2020, ~25 recent downloads). → <https://crates.io/api/v1/crates/mujoco-sys> | No — use `mujoco-rs`. |
| OpenAI-compatible LLM client | `async-openai` | `0.42.0` (2026-09-09) | Very active (2.5M recent downloads). → <https://crates.io/api/v1/crates/async-openai> | Yes; configurable `base_url` for OpenAI-compatible endpoints. **[unverified]** whether every non-OpenAI server's edge behaviors are covered. |
| Web/HTTP | `axum` | `0.8.9` (2026-04-14) | Very active (115M recent downloads), Tokio project. → <https://crates.io/api/v1/crates/axum> | Yes. |
| WebSocket | `tokio-tungstenite` | `0.30.0` (2026-07-11) | Very active (71M recent downloads). → <https://crates.io/api/v1/crates/tokio-tungstenite> | Yes. |
| SQLite | `rusqlite` | `0.40.2` (2026-08-08) | Very active (33M recent downloads). → <https://crates.io/api/v1/crates/rusqlite> | Yes. |

---

## C. ROS 2 distro / lifecycle

### C1. LTS status and EOL (as of 2026-09)

From `endoflife.date/ros-2` (last updated 2026-07-26), which tracks REP-2000:

| Release | Released | EOL | Type |
|---|---|---|---|
| **Lyrical Luth** | 2026-05-22 | **2031-05-31** | LTS (newest) |
| Kilted Kaiju | 2025-05-23 | 2026-12-31 | non-LTS |
| **Jazzy Jalisco** | 2024-05-23 | **2029-05-31** | LTS |
| Iron Irwini | 2023-05-23 | 2024-12-04 | non-LTS (EOL) |
| **Humble Hawksbill** | 2022-05-23 | **2027-05-31** | LTS (aging) |
| Galactic Geochelone | 2021-05-23 | 2022-12-09 | non-LTS (EOL) |
| **Foxy Fitzroy** | 2020-06-05 | **2023-06-20** | **EOL** |

→ <https://endoflife.date/ros-2>

REP-2000 confirms the cadence and support rules: annual May releases; even-year releases are LTS supported 5 years; odd-year releases are non-LTS supported 1.5 years; each ROS release targets exactly one Ubuntu LTS; releases do not add new Ubuntu support after launch; Foxy's window is documented as "May 2020 – May 2023". → <https://github.com/ros-infrastructure/rep/blob/master/rep-2000.rst>

**Ubuntu pairing:** Foxy→20.04; Humble→22.04; Jazzy→24.04; Lyrical→24.04. → REP-2000 platform tables, <https://github.com/ros-infrastructure/rep/blob/master/rep-2000.rst>

**Recommendation for an industrial robot in 2026:**
- **Jazzy on Ubuntu 24.04** is the pragmatic long-term target: mature, LTS through **May 2029**, and supported by `rclrs`.
- **Humble** is only a short bridge (EOL **May 2027**) and, while it is Unitree's *official* GO2 recommendation, it would mean a second migration within ~8 months.
- **Kilted** is not for production (EOL Dec 2026).
- **Lyrical Luth** (LTS to 2031) is the next refresh target, but it is brand-new and Unitree/rclrs ecosystem validation will lag. Plan it as a *future* hop, not today's.
- **Foxy is already EOL** (since 2023) and running an unsupported distro is an industrial risk.

### C2. `rclrs` support vs these distros

`rclrs`/`ros2_rust` supports **Humble, Jazzy, Kilted, Lyrical, Rolling**; **not Foxy**. → <https://github.com/ros2-rust/ros2_rust>. `ros2-client` (non-rclrs) similarly supports Humble→Lyrical via feature flags, default Jazzy. → <https://crates.io/api/v1/crates/ros2-client>.

There is a clean overlap: **Humble and Jazzy are the only distros supported by both Unitree (Humble official; Jazzy community) and `rclrs`.** Jazzy is the best target.

---

## D. Industrial deployment best practices (multi-package ROS 2 → edge)

### D1. Building `.deb`s from a ROS 2 workspace

The **current canonical, official ROS 2 approach** (as of the rolling docs) is still **`bloom` + `fakeroot debian/rules binary`**, applied per package:

```console
$ sudo apt install python3-bloom python3-rosdep fakeroot debhelper dh-python
$ sudo rosdep init && rosdep update
$ cd /path/to/pkg_source       # directory containing package.xml
$ bloom-generate rosdebian
$ fakeroot debian/rules binary
```
The `.deb` lands in the parent directory. → <https://github.com/ros2/ros2_documentation/blob/rolling/source/Developer-Tools/Build/Building-a-Custom-Deb-Package.rst>

- `bloom` is the release-automation tool that generates Debian packaging from a `package.xml`-based source tree; current PyPI version **0.14.4**. → <https://github.com/ros-infrastructure/bloom>, <https://pypi.org/pypi/bloom/json>
- For a *whole distribution* (many interdependent packages × distros × architectures), the ROS project itself uses **`ros_buildfarm`** — Docker/Jenkins-based job generation driven by REP-143 and `ros_buildfarm_config`. → <https://github.com/ros-infrastructure/ros_buildfarm>
- There is **no widely adopted "`colcon-deb`"** on PyPI (the `colcon-deb` PyPI name does not resolve), and ROS docs do not endorse one. If you see references to it, treat as **[unverified]**. The mature options are `bloom` (per-package) and `ros_buildfarm` (fleet).

**Practical recipe for this repo:** one `bloom-generate rosdebian` + `fakeroot` build per ROS package (and a separate Debian package for Rust binaries, e.g. built with `cargo-deb`), rather than trying to `.deb`-ify the whole workspace as one blob.

### D2. Hosting a signed apt repository and pinning

- **reprepro** manages a pool of `.deb`/`.udeb`/`.dsc`, stores checksums in a local DB, "checking signatures of mirrored repositories and creating signatures of the generated Package indices is supported." → <https://manpages.debian.org/bookworm/reprepro/reprepro.1.en.html>
- **aptly** adds an explicit snapshot model for repeatability: mirrors, local repos, **immutable snapshots**, and published repos, so "package installation and upgrade becomes deterministic" and you can roll back. → <https://www.aptly.info/doc/overview/>
- **Pinning:** publish immutable snapshots per release and pin with apt `Pin-Priority`/`/etc/apt/preferences.d` (version pinning). *[The apt pinning mechanics are standard Debian; cite Debian `apt_preferences(5)` for the exact syntax — not fetched here.]*
- Signing: both tools support GPG-signed `Release`/`InRelease` metadata; reprepro explicitly supports creating signatures. → <https://manpages.debian.org/bookworm/reprepro/reprepro.1.en.html>

**Recommendation:** aptly for deterministic snapshot promotion + rollback; reprepro is a lighter, equally valid choice. Serve over TLS, sign metadata, and pin exact versions per fleet.

### D3. systemd service units, config placement, logging

- Package each ROS 2 node as a **systemd service** (`/etc/systemd/system/<node>.service`, `Type=` appropriate, `ExecStart=` the installed binary/launch). → <https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html>
- **Secrets/config:** prefer **systemd credentials** over env files for keys/tokens. Credentials are acquired at service activation, exposed as files under `$CREDENTIALS_DIRECTORY`, not inherited by child processes, access-checked by the kernel, optionally TPM2-encrypted (`LoadCredentialEncrypted=`, `systemd-creds encrypt`). → <https://systemd.io/CREDENTIALS/>
- **Non-secret config** belongs in `/etc/<pkg>/` (or `/usr/lib/...` defaults + `/etc` overrides). Store mutable state on a separate data partition for OTA reasons (see D4).
- **Logging:** run under systemd and send logs to **journald** (`journalctl -u <node>`); avoid ad-hoc log files on the rootfs so they survive read-only/A-B updates.

### D4. A/B OTA / atomic updates: RAUC vs SWUpdate vs Mender vs OSTree

| Tool | Model | Key properties | Source |
|---|---|---|---|
| **RAUC** | Lightweight update client + host bundling tool; integrates with bootloaders (GRUB/U-Boot/barebox/EFI) | "Fail-Safe & Atomic"; interrupted updates don't brick; compatibility check; boot success/failure marking; OpenSSL/x.509 signed bundles; HTTP(S) streaming; D-Bus API; Yocto/Buildroot/PTXdist support | <https://rauc.readthedocs.io/en/latest/> |
| **SWUpdate** | Framework (handlers/parsers) for embedded updates; single `.swu` cpio image + `sw-description` | A/B "software collections" + single-copy; image authenticated & verified; power-off safe; U-Boot/GRUB/EFI Boot Guard env support; optional systemd integration; hawkBit backend; Debian/Ubuntu packages available | <https://sbabic.github.io/swupdate/swupdate.html> |
| **Mender** | Client/server fleet management; dual A/B with commit/rollback; Update Modules for app updates; can combine OS+app updates | Central server, managed or standalone mode; robust A/B rollback; signed artifacts; delta updates; add-ons (remote terminal, monitor) | <https://docs.mender.io/overview/introduction> |
| **OSTree / libostree** | Git-like content-addressed bootable filesystem trees; transactional upgrades + rollback | Incremental HTTP + GPG; "support for parallel installing more than just 2 bootable roots"; used by Fedora CoreOS/IoT, Torizon, Apertis; Debian support via external tooling (`apt2ostree`, deb-ostree-builder) | <https://ostreedev.github.io/ostree/> |

**Recommendation:**
- If you control the full OS image and want minimal dependencies on an Ubuntu-based robot: **RAUC** (simple, signed, atomic, bootloader-agnostic) — the best fit for a single-robot/small-fleet product.
- If you need a **managed fleet with a server, inventory, and remote operations UI**: **Mender** (or RAUC + `rauc-hawkbit-updater` → hawkBit).
- **SWUpdate** is the strongest when you already have a Yocto/Buildroot BSP and want handler-based partial updates.
- **OSTree** makes sense if you want an image-based, immutable OS (e.g. Fedora IoT/Torizon-style); on Ubuntu it requires extra tooling and is the least turnkey here.

**Critical design constraint (from Mender):** OS A/B updates replace the whole root filesystem, so "to be updatable, a filesystem needs to be stateless" — all persistent/mutable data must live on a separate partition. → <https://docs.mender.io/overview/introduction>

### D5. Security (SROS2), DDS discovery over networks, QoS for control loops

**SROS2.** ROS 2 security is implemented via DDS-Security plugins and is **off by default**; enable with `ROS_SECURITY_ENABLE=true` and optionally enforce with `ROS_SECURITY_STRATEGY=Enforce`. Each enclave needs six files: `identity_ca.cert.pem`, `cert.pem`, `key.pem`, `permissions_ca.cert.pem`, `governance.p7s`, `permissions.p7s`. → <https://github.com/ros2/ros2_documentation/blob/rolling/source/Developer-Tools/Introspection-and-analysis/Security/About-Security.rst>. `sros2` provides the key/distribution tooling and is tested against FastDDS, CycloneDDS, and Connext. → <https://github.com/ros2/sros2>. **Caveat:** if you also run Unitree's own nodes, verify the Unitree stack tolerates DDS-Security (unverified) — you may need security only on the control subnet, not the robot link.

**DDS discovery over networks.** DDS uses distributed RTPS discovery; nodes on the same domain discover automatically, but this can be constrained by interface and peer configuration. Unitree deployments pin the interface (`CYCLONEDDS_URI` `<NetworkInterface>`) and can add explicit `<Peers>` (as community GO2 configs do), which is the correct pattern when multicast does not cross subnets. → <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/setup.sh>, <https://github.com/jkk-research/JKK_UNITREE_GO2>, <https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/client-libraries/About-Different-Middleware-Vendors.rst>. Keep the robot's DDS domain isolated (domain 0 default; change with `ROS_DOMAIN_ID`).

**QoS for control loops.** The default ROS 2 profile is KeepLast(10)/Reliable/Volatile. For sensor streams (camera, LiDAR) use **best-effort + small depth**; for commands/state use **reliable**. Volatile vs transient-local matters for late joiners (latched topics need both sides TransientLocal). QoS compatibility is a request/offer model; a mismatch silently yields **no messages**. Use **Deadline/Liveliness** and the incompatible-QoS events to detect stalled control loops. → <https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/interfaces/topics/About-Quality-of-Service-Settings.rst>, <https://design.ros2.org/articles/qos.html>. Unitree low-level control is a hard real-time loop (example runs a 0.002 s / 500 Hz writer thread), so keep the lowcmd path on a dedicated, reliably-configured channel and avoid sharing an executor with heavy vision work. → <https://raw.githubusercontent.com/unitreerobotics/unitree_sdk2_python/master/example/go2/low_level/go2_stand_example.py>

### D6. Reproducible builds and CI

- **Build in Docker** (pinned base image = target Ubuntu/ROS distro) so that `.deb`s and binaries are reproducible and match the robot's runtime. ROS' own infrastructure is Docker-based. → <https://github.com/ros-infrastructure/ros_buildfarm>
- **`colcon build` + `colcon test`** per package in CI; the Rust path adds `colcon-cargo`/`colcon-ros-cargo`, currently with known workarounds. → <https://github.com/ros2-rust/ros2_rust>
- **`ros_buildfarm`** is the reference for building and testing an entire distro across platforms; for a private product, use it as a design template (Docker per step, REP-143 distro config) rather than standing up the full Jenkins farm. → <https://github.com/ros-infrastructure/ros_buildfarm>
- Sign `.deb`s and repository metadata; record exact package versions per release snapshot (see D2) to make the robot's software set reproducible and rollback-able.

---

## Bottom line

### 1) Is the GO2 tied to Foxy, or can we move to Humble/Jazzy?

**Not tied to Foxy.** The robot coupling is **CycloneDDS 0.10.2 + Unitree IDL/topics + domain id + network interface**, all below the ROS-distro line. Unitree's own `unitree_ros2` README tests and **recommends Ubuntu 22.04/Humble**, and the same codebase supports both Foxy and Humble by sourcing a different ROS environment (single `master` branch, CI for `build-foxy` and `build-humble` only). Foxy has been EOL since 2023.

- **Humble:** officially supported by Unitree → low risk, but only until **May 2027**.
- **Jazzy:** community-supported (open, unmerged Unitree PR; no Jazzy CI), technically sound because it's the same DDS layer, but **not officially validated** → plan a bring-up/validation phase.
- **Recommendation:** **migrate to Jazzy (Ubuntu 24.04)**, skipping Humble as a permanent home to avoid a second migration in ~8 months; keep Humble only as a fallback if the Jazzy GO2 spike fails.

### 2) Is a Rust migration viable/recommended, and for which layers?

**Viable in layers; not as a big-bang rewrite.**

- **Recommended Rust layers:** non-ROS-bound application/edge services — LLM client (`async-openai`), web/WS API (`axum` + `tokio-tungstenite`), persistence (`rusqlite`), camera capture (`nokhwa`/`opencv`), AprilTag/vision (`opencv`, or `apriltag` accepting its staleness), and simulation (`mujoco-rs`). These are mature, actively maintained crates and do not touch the brittle robot interface.
- **Possible but de-risked with a spike:** replacing `unitree_sdk2_python` with a **direct DDS client** (`eclipse-cyclonedds` — same C library, or `rustdds`/`ros2-client`). The proof that pure-DDS access works without ROS is the Python SDK itself (CycloneDDS only, no `rclpy`). The risk is exact DDS type/XTypes matching for Unitree's `unitree_go`/`unitree_api` types — treat as an experiment, not a given.
- **Use `rclrs` only for Humble+ nodes** (never Foxy), understanding it is pre-1.0, has **no stability guarantees**, and needs extra build tooling.
- **Keep the real-time lowcmd/sport control path where Unitree validates it** (the C++ SDK or Python SDK on the supported distro) until a Rust DDS spike proves out. Do not put the 500 Hz control loop on an unproven stack.
- **Overall:** a hybrid **C++/Python (robot I/O) + Rust (application services)** architecture is the pragmatic, low-risk outcome. Full Rust is a multi-phase goal contingent on the DDS interoperability spike succeeding.

### 3) Recommended deployment model

1. **Runtime:** ROS 2 **Jazzy on Ubuntu 24.04** (LTS to 2029), `rmw_cyclonedds_cpp`, CycloneDDS pinned to the robot interface, DDS domain isolated.
2. **Packaging:** per-package `.deb`s via **`bloom-generate rosdebian` + `fakeroot debian/rules binary`**; Rust binaries via `cargo-deb`. Use `ros_buildfarm` as the pattern if the package count grows.
3. **Distribution:** a **signed apt repo (aptly snapshots, or reprepro)** with **version pinning**, serving immutable snapshots per release for deterministic installs and rollbacks.
4. **Runtime supervision:** one **systemd service per node**, config in `/etc`, **secrets via systemd credentials** (TPM2-encrypted), logs to **journald**.
5. **Updates:** **A/B image updates via RAUC or Mender** (signed, atomic, rollback-capable); keep all mutable state on a separate partition because OS updates replace the rootfs. Use apt/repo delivery only for application-layer packages inside a controlled image.
6. **Security/robustness:** SROS2 enforced on the control network; best-effort QoS for sensor streams and reliable+deadline/liveliness for control; monitor incompatible-QoS events; isolated DDS domains.
7. **CI:** Docker-pinned builds matching the robot image, `colcon build`/`colcon test`, signed artifacts, versioned release snapshots.

---

## Source index (primary unless noted)

**Unitree**
- unitree_ros2 README / setup / CHANGELOG / version: <https://github.com/unitreerobotics/unitree_ros2>, <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/setup.sh>, <https://github.com/unitreerobotics/unitree_ros2/blob/master/CHANGELOG.md>, <https://raw.githubusercontent.com/unitreerobotics/unitree_ros2/master/version.txt>
- unitree_ros2 branches: <https://api.github.com/repos/unitreerobotics/unitree_ros2/branches>
- unitree_ros2 CI workflows: <https://github.com/unitreerobotics/unitree_ros2/tree/master/.github/workflows>
- unitree_ros2 PR #3 (Jazzy, open): <https://github.com/unitreerobotics/unitree_ros2/pull/3>; PR #20 (Humble CI, merged): <https://github.com/unitreerobotics/unitree_ros2/pull/20>
- unitree_sdk2 README: <https://github.com/unitreerobotics/unitree_sdk2>
- unitree_sdk2 channel factory: <https://github.com/unitreerobotics/unitree_sdk2/blob/main/include/unitree/robot/channel/channel_factory.hpp>
- unitree_sdk2_python README / setup.py / pyproject / channel.py / channel_config.py / channel_name.py / go2 examples: <https://github.com/unitreerobotics/unitree_sdk2_python>, and `raw.githubusercontent.com/.../master/...` paths cited inline
- Unitree developer docs entry point: <https://support.unitree.com/home/en/developer/Quick_start>
- Community GO2 EDU/NX guide: <https://github.com/jkk-research/JKK_UNITREE_GO2>
- Partner GO2 onboarding (onboard PC): <https://docs.quadruped.de/projects/go2/html/go2_ros2_installation.html>

**Rust**
- ros2_rust / rclrs: <https://github.com/ros2-rust/ros2_rust>, <https://crates.io/api/v1/crates/rclrs>
- cyclonedds-rust: <https://github.com/eclipse-cyclonedds/cyclonedds-rust>, <https://crates.io/api/v1/crates/eclipse-cyclonedds>
- RustDDS: <https://github.com/Atostek/RustDDS>; ros2-client: <https://crates.io/api/v1/crates/ros2-client>
- dust_dds: <https://crates.io/api/v1/crates/dust_dds>, <https://s2e-systems.com/products/dust-dds>
- crate metadata: <https://crates.io/api/v1/crates/{apriltag,opencv,nokhwa,v4l,mujoco-rs,mujoco-sys,async-openai,axum,tokio-tungstenite,rusqlite}>

**ROS 2 lifecycle / concepts**
- EOL data: <https://endoflife.date/ros-2>
- REP-2000: <https://github.com/ros-infrastructure/rep/blob/master/rep-2000.rst>
- QoS: <https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/interfaces/topics/About-Quality-of-Service-Settings.rst>, <https://design.ros2.org/articles/qos.html>
- Middleware/interop: <https://github.com/ros2/ros2_documentation/blob/rolling/source/ROS-Framework/client-libraries/About-Different-Middleware-Vendors.rst>
- Security: <https://github.com/ros2/ros2_documentation/blob/rolling/source/Developer-Tools/Introspection-and-analysis/Security/About-Security.rst>, <https://github.com/ros2/sros2>

**Deployment**
- Debian build: <https://github.com/ros2/ros2_documentation/blob/rolling/source/Developer-Tools/Build/Building-a-Custom-Deb-Package.rst>, <https://github.com/ros-infrastructure/bloom>, <https://pypi.org/pypi/bloom/json>
- Build farm: <https://github.com/ros-infrastructure/ros_buildfarm>
- Apt repos: <https://manpages.debian.org/bookworm/reprepro/reprepro.1.en.html>, <https://www.aptly.info/doc/overview/>
- systemd: <https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html>, <https://systemd.io/CREDENTIALS/>
- OTA: <https://rauc.readthedocs.io/en/latest/>, <https://sbabic.github.io/swupdate/swupdate.html>, <https://docs.mender.io/overview/introduction>, <https://ostreedev.github.io/ostree/>
