"""Regression checks against the owned EV21 bank and native804F835C lookup."""
import importlib.util
import unittest
from pathlib import Path

spec=importlib.util.spec_from_file_location('stage_se',Path(__file__).with_name('stage_se.py'))
stage=importlib.util.module_from_spec(spec);spec.loader.exec_module(stage)


class DefinitionTable(unittest.TestCase):
    def test_original_event_ids_do_not_shift_or_drop_the_last_program(self):
        raw=(stage.SE/'event/scene_mix7522.rsp').read_bytes()
        offset,size=stage.pack_members(raw)[1]
        block,sounds=stage.definitions(raw[offset:offset+size])
        self.assertEqual(block,7522)
        self.assertEqual(sorted(sounds),list(range(7)))
        for sound in [0,1,6]:
            self.assertEqual(sounds[sound]['wave_blocks'],[7522])
            self.assertEqual(sounds[sound]['volume'],127)
            self.assertEqual(sounds[sound]['plays'],[dict(wave=sound,delay=0,volume=127,pan=64,note=60)])


if __name__=='__main__':unittest.main()
