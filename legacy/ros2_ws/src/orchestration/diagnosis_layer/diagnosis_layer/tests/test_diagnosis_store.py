from __future__ import annotations

import importlib.util
import os
import sys
import tempfile
import time

# Load desc_layer.diagnosis_store in isolation (the desc_layer package __init__
# pulls in flask/ROS which aren't available in this test environment).
_DESC_PKG = os.path.normpath(os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "desc_layer", "desc_layer"))
_spec = importlib.util.spec_from_file_location(
    "diagnosis_store_test_shim", os.path.join(_DESC_PKG, "diagnosis_store.py"))
_module = importlib.util.module_from_spec(_spec)
sys.modules["diagnosis_store_test_shim"] = _module
_spec.loader.exec_module(_module)
DiagnosisRecord = _module.DiagnosisRecord
DiagnosisStore = _module.DiagnosisStore


def test_purge_older_than():
    d = tempfile.mkdtemp()
    store = DiagnosisStore(db_dir=d)
    old = DiagnosisRecord("old1", ["mock_spo2"], "periodic", created_at=time.time() - 100000)
    new = DiagnosisRecord("new1", ["mock_spo2"], "periodic", created_at=time.time())
    store.add(old)
    store.add(new)

    deleted = store.purge_older_than(3600)
    assert deleted == 1
    assert store.get("old1") is None
    assert store.get("new1") is not None


def test_purge_disabled_when_zero():
    d = tempfile.mkdtemp()
    store = DiagnosisStore(db_dir=d)
    store.add(DiagnosisRecord("x1", ["mock_spo2"], "periodic", created_at=time.time() - 100000))
    assert store.purge_older_than(0) == 0
    assert store.get("x1") is not None
