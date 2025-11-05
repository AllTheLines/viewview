export function tileKey(z: number, x: number, y: number) {
  return `${z}/${x}/${y}`;
}

export function getParentTile(z: number, x: number, y: number) {
  if (z === 0) {
    return null;
  }

  const parentZ = z - 1;
  const parentX = Math.floor(x / 2);
  const parentY = Math.floor(y / 2);
  return { z: parentZ, x: parentX, y: parentY };
}
