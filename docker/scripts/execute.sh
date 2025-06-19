#!/bin/bash

echo "Running command: cargo make --profile ${__FIELDX_MAKE_PROFILE__} ${__FIELDX_TASK__}"
echo "Toolchain      : ${__FIELDX_DEFAULT_TOOLCHAIN__}"

cargo make --profile ${__FIELDX_MAKE_PROFILE__} ${__FIELDX_TASK__}

rc=$?

if [ $rc -ne 0 ]; then
    echo "Exiting with error code: $rc"
    exit $rc
fi

# If the original target is update-versions then updates the outputs of uncompilable tests
if [ "${__FIELDX_TASK__}" = "test-compilation" -a "${TRYBUILD}" = "overwrite" ]; then
    echo "Updating uncompilable tests outputs..."
    rsync -av ./fieldx/tests/uncompilable/ /fieldx-host/tests/uncompilable/
fi