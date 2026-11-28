# Adapt x265's build/linux/multilib.sh for Apple's libtool. The standard vcpkg
# port installs only the eight-bit encoder. Frame also exports 10-bit HEVC.
foreach(depth IN ITEMS 10 12 8)
    set(build "${CURRENT_BUILDTREES_DIR}/${TARGET_TRIPLET}-${depth}bit")
    set(options -DEXPORT_C_API=OFF -DHIGH_BIT_DEPTH=ON)
    if(depth EQUAL 12)
        list(APPEND options -DMAIN12=ON)
    elseif(depth EQUAL 8)
        set(options -DEXPORT_C_API=ON -DLINKED_10BIT=ON -DLINKED_12BIT=ON "-DEXTRA_LIB=${lib10}")
    endif()
    vcpkg_execute_required_process(
        COMMAND "${CMAKE_COMMAND}" -S "${SOURCE_PATH}/source" -B "${build}" -G Ninja
            "-DCMAKE_MAKE_PROGRAM=${NINJA}" "-DCMAKE_OSX_ARCHITECTURES=${VCPKG_OSX_ARCHITECTURES}"
            "-DCMAKE_OSX_DEPLOYMENT_TARGET=${VCPKG_OSX_DEPLOYMENT_TARGET}" -DCMAKE_BUILD_TYPE=Release
            -DENABLE_CLI=OFF -DENABLE_SHARED=OFF -DENABLE_LIBNUMA=OFF -DENABLE_PIC=ON ${options}
        WORKING_DIRECTORY "${CURRENT_BUILDTREES_DIR}" LOGNAME "configure-${TARGET_TRIPLET}-${depth}bit"
    )
    vcpkg_execute_required_process(
        COMMAND "${CMAKE_COMMAND}" --build "${build}" --target x265-static --parallel "${VCPKG_CONCURRENCY}"
        WORKING_DIRECTORY "${build}" LOGNAME "build-${TARGET_TRIPLET}-${depth}bit"
    )
    set(lib${depth} "${build}/libx265.a")
endforeach()
vcpkg_execute_required_process(
    COMMAND /usr/bin/libtool -static -o "${CURRENT_PACKAGES_DIR}/lib/libx265.a" "${lib8}" "${lib10}" "${lib12}"
    WORKING_DIRECTORY "${CURRENT_BUILDTREES_DIR}" LOGNAME "combine-${TARGET_TRIPLET}"
)
