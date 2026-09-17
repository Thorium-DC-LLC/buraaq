cmake_minimum_required(VERSION 3.13)

if (NOT DEFINED ENV{PICO_SDK_PATH})
  message(FATAL_ERROR "Set PICO_SDK_PATH to a pico-sdk checkout")
endif()

include($ENV{PICO_SDK_PATH}/external/pico_sdk_import.cmake)

project(buraaq_board C CXX ASM)

set(PICO_BOARD pico_w CACHE STRING "Board")
pico_sdk_init()

add_executable(buraaq_board board_main.c)
target_link_libraries(buraaq_board pico_stdlib pico_cyw43_arch_none)
pico_enable_stdio_usb(buraaq_board 0)
pico_enable_stdio_uart(buraaq_board 0)
pico_add_extra_outputs(buraaq_board)
