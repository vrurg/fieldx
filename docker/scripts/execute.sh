#!/bin/bash

echo "Running command: cargo make --profile ${__FIELDX_MAKE_PROFILE__} ${__FIELDX_TASK__}"
echo "Toolchain      : ${__FIELDX_DEFAULT_TOOLCHAIN__}"

cargo make --profile ${__FIELDX_MAKE_PROFILE__} ${__FIELDX_TASK__}

rc=$?

if [ $rc -ne 0 ]; then
    echo "Exiting with error code: $rc"
    exit $rc
fi