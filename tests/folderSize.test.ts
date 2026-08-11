import { describe, expect, test } from "bun:test";
import {
	measuredContentsLabel,
	measuredSizeLabel,
	type Measurement,
} from "../frontend/src/lib/folderSize";

function measurementOf(fields: Partial<Measurement>): Measurement {
	return {
		id: "usage-1",
		path: "/home/moritz/Downloads",
		bytes: 0,
		files: 0,
		directories: 0,
		unreadable: 0,
		cancelled: false,
		...fields,
	};
}

describe("the measured size", () => {
	test("says it is still counting before the first total arrives", () => {
		expect(measuredSizeLabel(null, true)).toBe("Measuring…");
	});

	test("marks a running total as partial, so it is not read as the answer", () => {
		expect(measuredSizeLabel(measurementOf({ bytes: 2048 }), true)).toBe("2.0 KB so far");
	});

	test("a finished total stands on its own", () => {
		expect(measuredSizeLabel(measurementOf({ bytes: 2048 }), false)).toBe("2.0 KB");
	});
});

describe("the measured contents", () => {
	test("counts files and folders separately", () => {
		const measurement = measurementOf({ files: 4201, directories: 132 });
		expect(measuredContentsLabel(measurement)).toBe("4,201 files, 132 folders");
	});

	test("reads naturally when there is one of something", () => {
		const measurement = measurementOf({ files: 1, directories: 1 });
		expect(measuredContentsLabel(measurement)).toBe("1 file, 1 folder");
	});

	test("an empty folder still says so", () => {
		expect(measuredContentsLabel(measurementOf({}))).toBe("0 files, 0 folders");
	});

	test("what the walk could not read is reported rather than folded into the total", () => {
		const measurement = measurementOf({ files: 3, directories: 1, unreadable: 2 });
		expect(measuredContentsLabel(measurement)).toBe("3 files, 1 folder, 2 unreadable");
	});

	test("nothing measured yet reads as unknown", () => {
		expect(measuredContentsLabel(null)).toBe("—");
	});
});
