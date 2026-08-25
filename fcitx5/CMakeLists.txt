# Addon Fcitx5 của Onikey — lớp C++ mỏng + lõi Rust tĩnh.
#
#   cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build
#   sudo cmake --install build
#
# Yêu cầu libonikey_core.a đã build: (cd ../rust && cargo build --release)
cmake_minimum_required(VERSION 3.16)
project(fcitx5-onikey VERSION 1.0.4 LANGUAGES CXX)

find_package(Fcitx5Core REQUIRED)
include(FeatureSummary)

set(ONIKEY_CORE_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../rust/target/release/libonikey_core.a"
    CACHE FILEPATH "Đường dẫn libonikey_core.a")
if(NOT EXISTS "${ONIKEY_CORE_LIB}")
    message(FATAL_ERROR "Chưa có ${ONIKEY_CORE_LIB} — chạy: (cd ../rust && cargo build --release)")
endif()

add_library(onikey MODULE src/onikey.cpp)
set_target_properties(onikey PROPERTIES PREFIX "" CXX_STANDARD 20 CXX_STANDARD_REQUIRED ON)
target_include_directories(onikey PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../rust/onikey-core-ffi/include")
target_link_libraries(onikey PRIVATE Fcitx5::Core "${ONIKEY_CORE_LIB}")

install(TARGETS onikey DESTINATION "${FCITX_INSTALL_LIBDIR}/fcitx5")
install(FILES onikey-addon.conf DESTINATION "${FCITX_INSTALL_PKGDATADIR}/addon" RENAME onikey.conf)
install(FILES inputmethod/onikey.conf DESTINATION "${FCITX_INSTALL_PKGDATADIR}/inputmethod")
