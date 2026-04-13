import union from '@turf/union';
import type {
  Feature,
  FeatureCollection,
  GeoJsonProperties,
  MultiPolygon,
  Polygon,
} from 'geojson';
import type { LngLat } from 'maplibre-gl';
import proj4 from 'proj4';
import { ANGLE_SHIFT, aeqdProjectionString, rotate, toRadians } from './utils';

export type PolarSegments = { angleID: number; pairs: number[] };
export const DEFAULT_OPACITY = 0.5;

export class Viewshed {
  id: string;
  centre: LngLat;
  geoJSON:
    | FeatureCollection<Polygon>
    | Feature<Polygon | MultiPolygon, GeoJsonProperties>;
  scale: number;
  colour = '#00ff00';
  isVisible = true;
  isLocked = false;

  constructor(
    id: string,
    centre: LngLat,
    scale: number,
    polar_segments: PolarSegments[],
  ) {
    this.id = id;
    this.centre = centre;
    this.scale = scale;
    this.geoJSON = this.generateGeoJSON(polar_segments);
  }

  getViewshedLayerID() {
    return `viewshed-layer-${this.id}`;
  }
  getViewshedSourceID() {
    return `viewshed-source-${this.id}`;
  }

  generateGeoJSON(polar_segments: PolarSegments[]) {
    const features = [];
    for (const polar_segment of polar_segments) {
      for (let i = 0; i < polar_segment.pairs.length; i += 2) {
        const start = polar_segment.pairs[i] * this.scale;
        const end = start + polar_segment.pairs[i + 1] * this.scale;
        const latLonStart = this.polarDistanceToPair(
          polar_segment.angleID,
          start,
        );
        const latLonEnd = this.polarDistanceToPair(polar_segment.angleID, end);
        const polygon: Feature<Polygon> = {
          type: 'Feature',
          geometry: {
            type: 'Polygon',
            coordinates: [
              [
                latLonStart[0],
                latLonStart[1],
                latLonEnd[1],
                latLonEnd[0],
                latLonStart[0],
              ],
            ],
          },
          properties: {},
        };

        features.push(polygon);
      }
    }

    return {
      type: 'FeatureCollection',
      features: features,
    } as FeatureCollection<Polygon>;
  }

  unionGeoJSON() {
    if (this.geoJSON === undefined) return;
    const unioned = union(this.geoJSON as FeatureCollection<Polygon>);
    if (!unioned) return;
    this.geoJSON = unioned;
  }

  polarDistanceToPair(angleID: number, distance: number) {
    // TODO: Don't assume that the kernel is computing 360 angles.
    const angle = angleID - 90;

    const θ = toRadians(angle);
    const dx = distance * Math.cos(θ);
    const dy = distance * Math.sin(θ);
    const rotatedClockwiseAEQD = rotate(dx, dy, -0.6);
    const rotatedAntiAEQD = rotate(dx, dy, +0.6);

    const aeqd = aeqdProjectionString(this.centre.lng, this.centre.lat);

    const rotatedClockwiseLonLat = proj4(
      aeqd,
      proj4.WGS84,
      rotatedClockwiseAEQD,
    );
    const rotatedAntiLonLat = proj4(
      aeqd,
      '+proj=longlat +datum=WGS84 +no_defs',
      rotatedAntiAEQD,
    );

    return [rotatedClockwiseLonLat, rotatedAntiLonLat];
  }
}
