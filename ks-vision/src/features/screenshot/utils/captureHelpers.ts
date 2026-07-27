export const getAspectSize = (width: number, height: number, max: number): { width: number; height: number } => {
  if (width <= 0 || height <= 0) return { width: 0, height: 0 };
  const ratio = width / height;
  if (width > height) {
    return {
      width: Math.min(width, max),
      height: Math.min(width, max) / ratio,
    };
  } else {
    return {
      width: Math.min(height, max) * ratio,
      height: Math.min(height, max),
    };
  }
};
