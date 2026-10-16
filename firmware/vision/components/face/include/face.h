#pragma once

#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

#define FACE_KEYPOINTS 10

typedef struct face_detector face_detector_t;
typedef struct face_embedder face_embedder_t;

/* RGB565, big-endian, as the camera driver delivers it. */
typedef struct {
    uint8_t *data;
    uint16_t width;
    uint16_t height;
} face_image_t;

typedef struct {
    int16_t box[4];
    int16_t keypoints[FACE_KEYPOINTS];
    float score;
} face_box_t;

esp_err_t face_encode_jpeg(const face_image_t *image, uint8_t quality, uint8_t **jpeg, size_t *len);
void face_jpeg_free(uint8_t *jpeg);

face_detector_t *face_detector_new(void);
size_t face_detect(face_detector_t *detector, const face_image_t *image, face_box_t *out, size_t max);

face_embedder_t *face_embedder_new(void);
size_t face_embedding_len(face_embedder_t *embedder);
esp_err_t face_embed(face_embedder_t *embedder,
                     const face_image_t *image,
                     const int16_t keypoints[FACE_KEYPOINTS],
                     float *out,
                     size_t len);

#ifdef __cplusplus
}
#endif
