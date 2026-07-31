#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import os
import platform
import plistlib
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path


TOOL_DIR = Path(__file__).resolve().parent
CACHE_DIR = TOOL_DIR / "cache"
SOURCE_DIR = TOOL_DIR / "sources"
BUILD_DIR = TOOL_DIR / "build"
OUTPUT_DIR = TOOL_DIR / "output"
WHEEL_DIR = BUILD_DIR / "wheels"
HOST_F2PY_VIRTUAL_ENV = BUILD_DIR / "host-f2py-venv"
FRAMEWORK_NAME = "OperitPythonScientific"
FRAMEWORK_OUTPUT = OUTPUT_DIR / f"{FRAMEWORK_NAME}.xcframework"
PYTHON_VERSION = "3.13"
MINIMUM_IOS_VERSION = "13.0"
SCIPY_NUMPY_CONFIG_TOOL = TOOL_DIR / "scipy_numpy_config.py"
SCIPY_NUMPY_CONSTRAINTS = TOOL_DIR / "scipy-numpy-constraints.txt"
SCIPY_IOS_ACCELERATE_MESON_PATH = Path("scipy/meson.build")
SCIPY_F2PY_GENERATOR_PATH = Path("tools/generate_f2pymod.py")
SCIPY_PYTHRAN_PROGRAM_PATH = Path("meson.build")
SCIPY_ACCELERATE_PLATFORM_CHECK = """macOS13_3_or_later = false
if host_machine.system() == 'darwin'
  r = run_command('xcrun', '-sdk', 'macosx', '--show-sdk-version', check: true)
  sdkVersion = r.stdout().strip()

  macOS13_3_or_later = sdkVersion.version_compare('>=13.3')
endif
"""
SCIPY_IOS_ACCELERATE_PLATFORM_CHECK = """ios_accelerate = host_machine.system() == 'ios'
macOS13_3_or_later = false
if host_machine.system() == 'darwin'
  r = run_command('xcrun', '-sdk', 'macosx', '--show-sdk-version', check: true)
  sdkVersion = r.stdout().strip()

  macOS13_3_or_later = sdkVersion.version_compare('>=13.3')
endif
"""
SCIPY_ACCELERATE_VALIDATION = """if blas_name == 'accelerate'
  if not macOS13_3_or_later
    error('macOS Accelerate is only supported on macOS >=13.3')
  endif
  if cc.get_id() != 'clang'
    warning('accelerate may not be properly detected with non-native Apple compiler due to https://github.com/mesonbuild/meson/issues/13608')
  endif
  _args_blas_lapack += ['-DACCELERATE_NEW_LAPACK']
  generate_blas_wrappers = true
  accelerate_flag = '-a'
endif
"""
SCIPY_IOS_ACCELERATE_VALIDATION = """if blas_name == 'accelerate'
  if not ios_accelerate and not macOS13_3_or_later
    error('Accelerate is only supported on iOS or macOS >=13.3')
  endif
  if cc.get_id() != 'clang'
    warning('accelerate may not be properly detected with non-native Apple compiler due to https://github.com/mesonbuild/meson/issues/13608')
  endif
  if not ios_accelerate
    _args_blas_lapack += ['-DACCELERATE_NEW_LAPACK']
    generate_blas_wrappers = true
  endif
  accelerate_flag = '-a'
endif
"""
SCIPY_F2PY_COMMAND = """            ['f2py', fname_pyf, '--build-dir', outdir_abs] + nogil_arg,
"""
SCIPY_HOST_F2PY_COMMAND = """            [os.environ['SCIPY_HOST_F2PY'], fname_pyf, '--build-dir', outdir_abs] + nogil_arg,
"""
SCIPY_PYTHRAN_PROGRAM = """  pythran = find_program('pythran', native: true, version: '>=0.14.0')
"""
SCIPY_HOST_PYTHRAN_PROGRAM = """  pythran = find_program(
    meson.get_external_property('pythran-program', 'not-given'),
    native: true,
    version: '>=0.14.0',
  )
"""
@dataclass(frozen=True)
class SourcePackage:
    """Describes one source distribution compiled into the embedded scientific runtime."""

    name: str
    version: str
    url: str
    sha256: str
    archive_root: str


NUMPY = SourcePackage(
    name="numpy",
    version="2.2.3",
    url=(
        "https://files.pythonhosted.org/packages/fb/90/8956572f5c4ae52201fdec7ba2044b2c"
        "882832dcec7d5d0922c9e9acf2de/numpy-2.2.3.tar.gz"
    ),
    sha256="dbdc15f0c81611925f382dfa97b3bd0bc2c1ce19d4fe50482cb0ddc12ba30020",
    archive_root="numpy-2.2.3",
)
SCIPY = SourcePackage(
    name="scipy",
    version="1.15.2",
    url=(
        "https://files.pythonhosted.org/packages/b7/b9/31ba9cd990e626574baf93fbc1ac61cf"
        "9ed54faafd04c479117517661637/scipy-1.15.2.tar.gz"
    ),
    sha256="cd58a314d92838f7e6f755c8a2167ead4f27e1fd5c1251fd54289569ef3495ec",
    archive_root="scipy-1.15.2",
)
IOS_BUILD_TAGS = (
    "cp313-ios_arm64_iphoneos",
    "cp313-ios_arm64_iphonesimulator",
    "cp313-ios_x86_64_iphonesimulator",
)
CIBW_CONFIG_SETTINGS_BY_PACKAGE = {
    "numpy": (
        "setup-args=-Duse-ilp64=false "
        "setup-args=-Dallow-noblas=false "
        "setup-args=-Dblas=accelerate "
        "setup-args=-Dlapack=accelerate "
        "build-dir=build"
    ),
    "scipy": (
        "setup-args=-Dblas=accelerate "
        "setup-args=-Dlapack=accelerate "
        "build-dir=build"
    ),
}


def file_sha256(path: Path) -> str:
    """Calculates the SHA-256 digest of one local file."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(command: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    """Runs one required native scientific build command without suppressing failures."""
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def require_macos_xcode() -> None:
    """Requires the macOS Xcode SDK needed to build signed iOS native extension slices."""
    if platform.system() != "Darwin":
        raise RuntimeError("iOS NumPy/SciPy builds require macOS with Xcode")
    missing = [command for command in ["xcrun", "xcodebuild", "lipo"] if shutil.which(command) is None]
    if missing:
        raise RuntimeError("iOS NumPy/SciPy build tools are missing: " + ", ".join(missing))


def source_archive_path(source: SourcePackage) -> Path:
    """Returns the cache path used by one pinned scientific source archive."""
    return CACHE_DIR / f"{source.name}-{source.version}.tar.gz"


def download_source(source: SourcePackage) -> Path:
    """Downloads and verifies one pinned scientific source archive."""
    target = source_archive_path(source)
    if not target.is_file() or file_sha256(target) != source.sha256:
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_suffix(".part")
        temporary.unlink(missing_ok=True)
        with urllib.request.urlopen(source.url, timeout=120) as response:
            with temporary.open("wb") as output:
                shutil.copyfileobj(response, output, length=1024 * 1024)
        actual_sha256 = file_sha256(temporary)
        if actual_sha256 != source.sha256:
            raise RuntimeError(
                f"scientific source checksum is invalid: {source.name} "
                f"expected={source.sha256} actual={actual_sha256}"
            )
        temporary.replace(target)
    return target


def extract_archive(source: SourcePackage, destination: Path) -> Path:
    """Extracts a verified archive while requiring every entry to remain within its declared root."""
    target = destination / source.archive_root
    if target.exists():
        if target.is_symlink() or not target.is_dir():
            raise RuntimeError(f"scientific source target is not a directory: {target}")
        shutil.rmtree(target)
    target.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(download_source(source), "r:gz") as archive:
        members = archive.getmembers()
        roots = {member.name.split("/", 1)[0] for member in archive.getmembers() if member.name}
        expected_root = source.archive_root
        if roots != {expected_root}:
            raise RuntimeError(f"scientific source root is invalid: {source.name}")
        resolved_destination = destination.resolve()
        resolved_target = target.resolve()
        for member in members:
            member_destination = (destination / member.name).resolve()
            if (
                resolved_destination not in member_destination.parents
                and member_destination != resolved_destination
            ):
                raise RuntimeError(f"scientific source entry escapes build directory: {member.name}")
            if member.islnk() or (not member.isfile() and not member.isdir() and not member.issym()):
                raise RuntimeError(f"scientific source contains unsupported member: {member.name}")
            if member.issym():
                link_destination = (member_destination.parent / member.linkname).resolve()
                if (
                    resolved_target not in link_destination.parents
                    and link_destination != resolved_target
                ):
                    raise RuntimeError(
                        f"scientific source symbolic link escapes source tree: {member.name}"
                    )
        archive.extractall(destination, members=members, filter="data")
    return target


def replace_required_source_text(path: Path, expected: str, replacement: str, description: str) -> None:
    """Replaces one exact upstream source block and rejects any source revision drift."""
    contents = path.read_text(encoding="utf-8")
    matches = contents.count(expected)
    if matches != 1:
        raise RuntimeError(f"unexpected SciPy {description} block count: {matches}")
    path.write_text(contents.replace(expected, replacement), encoding="utf-8")


def configure_scipy_ios_accelerate(source: Path) -> None:
    """Configures SciPy to use the iOS 13-compatible LP64 Accelerate ABI."""
    meson_build = source / SCIPY_IOS_ACCELERATE_MESON_PATH
    if not meson_build.is_file():
        raise RuntimeError(f"SciPy Accelerate configuration file is missing: {meson_build}")
    replace_required_source_text(
        meson_build,
        SCIPY_ACCELERATE_PLATFORM_CHECK,
        SCIPY_IOS_ACCELERATE_PLATFORM_CHECK,
        "Accelerate platform check",
    )
    replace_required_source_text(
        meson_build,
        SCIPY_ACCELERATE_VALIDATION,
        SCIPY_IOS_ACCELERATE_VALIDATION,
        "Accelerate validation",
    )
def configure_scipy_host_f2py(source: Path) -> None:
    """Requires SciPy's F2Py generator subprocess to use the native host executable."""
    generator = source / SCIPY_F2PY_GENERATOR_PATH
    if not generator.is_file():
        raise RuntimeError(f"SciPy F2Py generator is missing: {generator}")
    replace_required_source_text(
        generator,
        SCIPY_F2PY_COMMAND,
        SCIPY_HOST_F2PY_COMMAND,
        "F2Py generator command",
    )


def configure_scipy_host_pythran(source: Path) -> None:
    """Requires SciPy's Meson project to use the native host Pythran executable."""
    meson_build = source / SCIPY_PYTHRAN_PROGRAM_PATH
    if not meson_build.is_file():
        raise RuntimeError(f"SciPy top-level Meson file is missing: {meson_build}")
    replace_required_source_text(
        meson_build,
        SCIPY_PYTHRAN_PROGRAM,
        SCIPY_HOST_PYTHRAN_PROGRAM,
        "Pythran program selection",
    )


def extract_source(package: SourcePackage) -> Path:
    """Extracts one package source and applies required cross-build adjustments."""
    target = extract_archive(package, SOURCE_DIR)
    if package.name == SCIPY.name:
        configure_scipy_ios_accelerate(target)
        configure_scipy_host_f2py(target)
        configure_scipy_host_pythran(target)
    return target


def build_wheels(
    package: SourcePackage,
    source: Path,
    build_python: Path,
    host_f2py: Path,
    host_pythran: Path,
) -> None:
    """Builds all device and simulator wheels for one scientific source distribution."""
    env = os.environ.copy()
    env["CIBW_BUILD"] = " ".join(IOS_BUILD_TAGS)
    env["CIBW_ARCHS_IOS"] = "arm64_iphoneos arm64_iphonesimulator x86_64_iphonesimulator"
    env["CIBW_BEFORE_BUILD"] = ""
    env["CIBW_XBUILD_TOOLS"] = "cmake ninja"
    env["CIBW_CONFIG_SETTINGS"] = CIBW_CONFIG_SETTINGS_BY_PACKAGE[package.name]
    if package.name == SCIPY.name:
        cross_file = BUILD_DIR / "scipy-numpy-config-cross-file.ini"
        env["CIBW_BEFORE_BUILD"] = (
            f"python {SCIPY_NUMPY_CONFIG_TOOL} --prepare-cross-build "
            f"{WHEEL_DIR} {cross_file} {host_f2py} {host_pythran}"
        )
        env["CIBW_CONFIG_SETTINGS"] += f" setup-args=--cross-file={cross_file}"
    env["CIBW_ENVIRONMENT"] = (
        "NPY_BLAS_ORDER=accelerate "
        "NPY_LAPACK_ORDER=accelerate "
        f"IPHONEOS_DEPLOYMENT_TARGET={MINIMUM_IOS_VERSION}"
    )
    if package.name == SCIPY.name:
        env["CIBW_ENVIRONMENT"] += (
            f" PIP_CONSTRAINT={SCIPY_NUMPY_CONSTRAINTS} SCIPY_HOST_F2PY={host_f2py}"
        )
    env["CIBW_TEST_SKIP"] = "*"
    run(
        [
            str(build_python),
            "-m",
            "cibuildwheel",
            "--platform",
            "ios",
            "--output-dir",
            str(WHEEL_DIR),
            str(source),
        ],
        env=env,
    )


def prepare_cibuildwheel_python() -> Path:
    """Creates the local virtual environment used exclusively for cibuildwheel tooling."""
    virtual_environment = BUILD_DIR / "cibuildwheel-venv"
    run([sys.executable, "-m", "venv", str(virtual_environment)])
    python = virtual_environment / "bin" / "python"
    if not python.is_file():
        raise RuntimeError(f"cibuildwheel virtual environment is missing Python: {python}")
    run([str(python), "-m", "pip", "install", "cibuildwheel==4.1.1"])
    return python


def prepare_host_f2py() -> Path:
    """Creates the native NumPy code generators required by the SciPy cross build."""
    host_python = shutil.which(f"python{PYTHON_VERSION}")
    if host_python is None:
        raise RuntimeError(f"macOS host Python {PYTHON_VERSION} is required for F2Py generation")
    shutil.rmtree(HOST_F2PY_VIRTUAL_ENV, ignore_errors=True)
    run([host_python, "-m", "venv", str(HOST_F2PY_VIRTUAL_ENV)])
    python = HOST_F2PY_VIRTUAL_ENV / "bin" / "python"
    if not python.is_file():
        raise RuntimeError(f"host F2Py virtual environment is missing Python: {python}")
    run(
        [
            str(python),
            "-m",
            "pip",
            "install",
            f"numpy=={NUMPY.version}",
            "pythran==0.17.0",
        ]
    )
    f2py = HOST_F2PY_VIRTUAL_ENV / "bin" / "f2py"
    if not f2py.is_file():
        raise RuntimeError(f"host NumPy F2Py executable is missing: {f2py}")
    return f2py


def host_pythran_path() -> Path:
    """Resolves the native Pythran executable installed beside host F2Py."""
    pythran = HOST_F2PY_VIRTUAL_ENV / "bin" / "pythran"
    if not pythran.is_file():
        raise RuntimeError(f"host Pythran executable is missing: {pythran}")
    return pythran


def wheel_path(package: SourcePackage, build_tag: str) -> Path:
    """Resolves one mandatory wheel selected by package and cibuildwheel iOS build tag."""
    candidates = sorted(WHEEL_DIR.glob(f"{package.name}-{package.version}-*{build_tag}*.whl"))
    if len(candidates) != 1:
        raise RuntimeError(
            f"expected one {package.name} wheel for {build_tag}, found {len(candidates)}"
        )
    return candidates[0]


def extract_wheels(packages: list[SourcePackage], build_tag: str, destination: Path) -> None:
    """Extracts the exact package wheels for one iOS architecture into site-packages."""
    destination.mkdir(parents=True, exist_ok=True)
    for package in packages:
        with zipfile.ZipFile(wheel_path(package, build_tag)) as wheel:
            for member in wheel.infolist():
                target = (destination / member.filename).resolve()
                if destination.resolve() not in target.parents and target != destination.resolve():
                    raise RuntimeError(f"scientific wheel entry escapes site-packages: {member.filename}")
            wheel.extractall(destination)


def framework_info() -> dict[str, object]:
    """Returns the canonical Info.plist contents for each embedded scientific framework slice."""
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleExecutable": FRAMEWORK_NAME,
        "CFBundleIdentifier": "com.ai.assistance.operit2.python-scientific",
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": FRAMEWORK_NAME,
        "CFBundlePackageType": "FMWK",
        "CFBundleShortVersionString": "1.0",
        "CFBundleVersion": "1",
        "MinimumOSVersion": MINIMUM_IOS_VERSION,
    }


def make_framework_binary(sdk: str, architectures: list[str], destination: Path) -> None:
    """Builds the dynamic marker binary that makes scientific resources a signed framework."""
    source = BUILD_DIR / "operit_python_scientific.c"
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_text("int operit_python_scientific_marker(void) { return 1; }\n", encoding="utf-8")
    minimum_version_flag = (
        "-mios-simulator-version-min=" + MINIMUM_IOS_VERSION
        if sdk == "iphonesimulator"
        else "-miphoneos-version-min=" + MINIMUM_IOS_VERSION
    )
    architecture_arguments = [argument for architecture in architectures for argument in ["-arch", architecture]]
    run(
        [
            "xcrun",
            "--sdk",
            sdk,
            "clang",
            "-dynamiclib",
            *architecture_arguments,
            minimum_version_flag,
            "-install_name",
            f"@rpath/{FRAMEWORK_NAME}.framework/{FRAMEWORK_NAME}",
            str(source),
            "-o",
            str(destination),
        ]
    )


def create_framework_slice(
    name: str,
    sdk: str,
    architectures: list[str],
    site_packages: Path,
) -> Path:
    """Creates one resource-bearing framework slice containing architecture-matched extensions."""
    framework = BUILD_DIR / name / f"{FRAMEWORK_NAME}.framework"
    shutil.rmtree(framework.parent, ignore_errors=True)
    resources = framework / "Resources" / "python" / "site-packages"
    resources.parent.mkdir(parents=True)
    shutil.copytree(site_packages, resources)
    with (framework / "Info.plist").open("wb") as stream:
        plistlib.dump(framework_info(), stream)
    make_framework_binary(sdk, architectures, framework / FRAMEWORK_NAME)
    return framework


def merge_simulator_extensions(arm64_site_packages: Path, x86_site_packages: Path, destination: Path) -> None:
    """Creates universal simulator extension modules by lipo-merging every native Python module."""
    shutil.copytree(arm64_site_packages, destination)
    arm64_extensions = sorted(path for path in destination.rglob("*.so") if path.is_file())
    for arm64_extension in arm64_extensions:
        relative = arm64_extension.relative_to(destination)
        x86_extension = x86_site_packages / relative
        if not x86_extension.is_file():
            raise RuntimeError(f"x86_64 simulator extension is missing: {relative}")
        universal_extension = arm64_extension.with_suffix(".universal")
        run(["lipo", "-create", str(arm64_extension), str(x86_extension), "-output", str(universal_extension)])
        universal_extension.replace(arm64_extension)


def create_xcframework(packages: list[SourcePackage]) -> None:
    """Creates a device and universal-simulator XCFramework carrying NumPy and SciPy extensions."""
    device_site_packages = BUILD_DIR / "device-site-packages"
    arm64_simulator_site_packages = BUILD_DIR / "arm64-simulator-site-packages"
    x86_simulator_site_packages = BUILD_DIR / "x86-simulator-site-packages"
    universal_simulator_site_packages = BUILD_DIR / "universal-simulator-site-packages"
    for directory in [
        device_site_packages,
        arm64_simulator_site_packages,
        x86_simulator_site_packages,
        universal_simulator_site_packages,
    ]:
        shutil.rmtree(directory, ignore_errors=True)
    extract_wheels(packages, IOS_BUILD_TAGS[0], device_site_packages)
    extract_wheels(packages, IOS_BUILD_TAGS[1], arm64_simulator_site_packages)
    extract_wheels(packages, IOS_BUILD_TAGS[2], x86_simulator_site_packages)
    merge_simulator_extensions(
        arm64_simulator_site_packages,
        x86_simulator_site_packages,
        universal_simulator_site_packages,
    )
    device_framework = create_framework_slice(
        "device", "iphoneos", ["arm64"], device_site_packages
    )
    simulator_framework = create_framework_slice(
        "simulator", "iphonesimulator", ["arm64", "x86_64"], universal_simulator_site_packages
    )
    shutil.rmtree(FRAMEWORK_OUTPUT, ignore_errors=True)
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    run(
        [
            "xcodebuild",
            "-create-xcframework",
            "-framework",
            str(device_framework),
            "-framework",
            str(simulator_framework),
            "-output",
            str(FRAMEWORK_OUTPUT),
        ]
    )


def verify_output() -> None:
    """Verifies that both generated framework slices carry NumPy and SciPy package resources."""
    required = [
        FRAMEWORK_OUTPUT / "Info.plist",
        FRAMEWORK_OUTPUT / "ios-arm64" / f"{FRAMEWORK_NAME}.framework" / FRAMEWORK_NAME,
        FRAMEWORK_OUTPUT
        / "ios-arm64_x86_64-simulator"
        / f"{FRAMEWORK_NAME}.framework"
        / FRAMEWORK_NAME,
    ]
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError("scientific XCFramework files are missing: " + ", ".join(missing))


def main() -> int:
    """Builds the complete signed-framework payload used by embedded iOS NumPy and SciPy."""
    require_macos_xcode()
    build_python = prepare_cibuildwheel_python()
    host_f2py = prepare_host_f2py()
    host_pythran = host_pythran_path()
    shutil.rmtree(WHEEL_DIR, ignore_errors=True)
    WHEEL_DIR.mkdir(parents=True)
    packages = [NUMPY, SCIPY]
    for package in packages:
        build_wheels(package, extract_source(package), build_python, host_f2py, host_pythran)
    create_xcframework(packages)
    verify_output()
    print(f"iOS scientific XCFramework: {FRAMEWORK_OUTPUT}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
