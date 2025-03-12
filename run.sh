#!/bin/bash

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Use the absolute path to the Python in the virtual environment
VENV_PYTHON="${SCRIPT_DIR}/venv/bin/python"

echo "Running with sudo as keyboard input capture requires root privileges on Linux"
"${VENV_PYTHON}" "${SCRIPT_DIR}/main.py" "$@"
