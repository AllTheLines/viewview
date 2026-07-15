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
import {
  aeqdProjectionString,
  getRandomColor,
  rotate,
  toRadians,
} from './utils';
import { reconstruct } from './viewshed-reconstructor/viewshed_reconstructor.js';

export type PolarSegments = { angle: number; bitpacks: number[] };
export const DEFAULT_OPACITY = 0.5;

export class Viewshed {
  id: string;
  centre: LngLat;
  geoJSON:
    | FeatureCollection<Polygon>
    | Feature<Polygon | MultiPolygon, GeoJsonProperties>;
  demScale: number;
  angleScale: number;
  colour = '#00ff00';
  isVisible = true;
  isLocked = false;

  constructor(
    id: string,
    centre: LngLat,
    demScale: number,
    angleScale: number,
    polar_segments: PolarSegments[],
  ) {
    this.id = id;
    this.centre = centre;
    this.demScale = demScale;
    this.angleScale = angleScale;
    this.geoJSON = this.generateGeoJSON(polar_segments);
  }

  getViewshedLayerID() {
    return `viewshed-layer-${this.id}`;
  }
  getViewshedSourceID() {
    return `viewshed-source-${this.id}`;
  }

  generateGeoJSON(polarSegments: PolarSegments[]) {
    const start = performance.now();

    try {
      const flattenedSegments = this.flattenSegmentsArray(polarSegments);
      const viewshed = this.buildViewshed(flattenedSegments);
      const end = performance.now();
      const duration = end - start;

      console.log(`Viewshed reconstructed in: ${duration}ms`);
      return viewshed;
    } catch (error) {
      console.error(
        'WASM viewshed reconstruction failed, falling back to non-unioned JS.',
        error,
      );
      return this.buildViewshedWithoutUnion(polarSegments);
    }
  }

  flattenSegmentsArray(polarSegments: PolarSegments[]): number[][] {
    polarSegments.sort((left, right) => left.angle - right.angle);
    const segmentsByAngle = [];
    for (const polarSegment of polarSegments) {
      const bitpacks = [];
      for (const bitpack of polarSegment.bitpacks) {
        bitpacks.push(bitpack);
      }
      segmentsByAngle.push(bitpacks);
    }

    return segmentsByAngle;
  }

  buildViewshed(flattenedSegments: number[][]) {
    const polygons = reconstruct(flattenedSegments, this.demScale);

    const features = [];

    for (const polygon of polygons) {
      const all = [];

      const exteriorCoords = [];
      for (let i = 0; i < polygon.exterior.length; i += 2) {
        const coordinate = [polygon.exterior[i], polygon.exterior[i + 1]];
        const projected = this.project(coordinate);
        exteriorCoords.push(projected);
      }
      if (exteriorCoords.length > 1) {
        exteriorCoords.push(exteriorCoords[0]);
      }
      all.push(exteriorCoords);

      for (let index = 0; index < polygon.interiors.length; index++) {
        const start = polygon.hole_indices[index];

        const end =
          index + 1 < polygon.hole_indices.length
            ? polygon.hole_indices[index + 1]
            : polygon.interiors.length;

        const holeCoords = [];
        for (let i = start; i < end; i += 2) {
          const coordinate = [polygon.interiors[i], polygon.interiors[i + 1]];
          const projected = this.project(coordinate);
          holeCoords.push(projected);
        }
        all.push(holeCoords);
      }

      const polygonGeoJSON: Feature<Polygon> = {
        type: 'Feature',
        geometry: {
          type: 'Polygon',
          coordinates: all,
        },
        properties: {
          color: getRandomColor(),
        },
      };

      features.push(polygonGeoJSON);

      polygon.free(); // Free WASM memory
    }

    return {
      type: 'FeatureCollection',
      features: features,
    } as FeatureCollection<Polygon>;
  }

  buildViewshedWithoutUnion(polarSegments: PolarSegments[]) {
    console.log(polarSegments);
    const features = [];
    for (const polarSegment of polarSegments) {
      for (let i = 0; i < polarSegment.bitpacks.length; i++) {
        const high16 = polarSegment.bitpacks[i] >>> 16;
        const low16 = polarSegment.bitpacks[i] & 0xffff;
        const start = high16 * this.demScale;
        const end = start + low16 * this.demScale;
        const latLonStart = this.polarDistanceToPair(polarSegment.angle, start);
        const latLonEnd = this.polarDistanceToPair(polarSegment.angle, end);
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

  project(coordinate: number[]) {
    const aeqd = aeqdProjectionString(this.centre.lng, this.centre.lat);
    return proj4(aeqd, proj4.WGS84, coordinate);
  }

  polarDistanceToPair(angle: number, distance: number) {
    const overlap = 1.1;
    const arc = 1 / (this.angleScale * overlap) / 2.0;
    const θ = toRadians(angle);
    const dx = distance * Math.cos(θ);
    const dy = distance * Math.sin(θ);
    const rotatedClockwiseAEQD = rotate(dx, dy, -arc);
    const rotatedAntiAEQD = rotate(dx, dy, +arc);

    const rotatedClockwiseLonLat = this.project(rotatedClockwiseAEQD);
    const rotatedAntiLonLat = this.project(rotatedAntiAEQD);

    return [rotatedClockwiseLonLat, rotatedAntiLonLat];
  }
}
