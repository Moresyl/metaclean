"""Build small, finite-loop demos from unmodified native captures (requires Pillow)."""
from pathlib import Path
from PIL import Image

assets = Path(__file__).resolve().parent.parent / "assets"
for locale in ("en", "zh"):
    frames = []
    for stage in ("home", "intake", "scan", "search", "clean"):
        with Image.open(assets / f"metaclean-{stage}-{locale}.png") as source:
            frame = source.convert("RGB").resize((944, 576), Image.Resampling.LANCZOS)
            frames.append(frame.quantize(colors=128))
    frames[0].save(
        assets / f"metaclean-workflow-{locale}.gif",
        save_all=True,
        append_images=frames[1:],
        duration=[1600, 1600, 2800, 2400, 2800],
        loop=1,
        optimize=True,
        disposal=2,
    )
    print(f"Built {locale} native workflow GIF")
