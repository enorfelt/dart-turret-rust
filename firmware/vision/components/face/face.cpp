#include "face.h"

#include <cstring>

#include "dl_image_jpeg.hpp"
#include "esp_heap_caps.h"
#include "human_face_detect.hpp"
#include "human_face_recognition.hpp"

struct face_detector {
    HumanFaceDetect model{HumanFaceDetect::MSRMNP_S8_V1, false};
};

struct face_embedder {
    HumanFaceFeat model{HumanFaceFeat::MFN_S8_V1, false};
};

static dl::image::img_t to_img(const face_image_t *image)
{
    return dl::image::img_t{
        .data = image->data,
        .width = image->width,
        .height = image->height,
        .pix_type = dl::image::DL_IMAGE_PIX_TYPE_RGB565BE,
    };
}

esp_err_t face_encode_jpeg(const face_image_t *image, uint8_t quality, uint8_t **jpeg, size_t *len)
{
    dl::image::jpeg_img_t encoded = dl::image::sw_encode_jpeg(to_img(image), quality);
    if (!encoded.data) {
        return ESP_FAIL;
    }
    *jpeg = static_cast<uint8_t *>(encoded.data);
    *len = encoded.data_len;
    return ESP_OK;
}

void face_jpeg_free(uint8_t *jpeg)
{
    heap_caps_free(jpeg);
}

face_detector_t *face_detector_new(void)
{
    return new face_detector();
}

size_t face_detect(face_detector_t *detector, const face_image_t *image, face_box_t *out, size_t max)
{
    auto &results = detector->model.run(to_img(image));
    size_t count = 0;
    for (const auto &result : results) {
        if (count == max) {
            break;
        }
        face_box_t &face = out[count++];
        for (int i = 0; i < 4; i++) {
            face.box[i] = static_cast<int16_t>(result.box[i]);
        }
        for (int i = 0; i < FACE_KEYPOINTS; i++) {
            face.keypoints[i] = i < static_cast<int>(result.keypoint.size()) ? result.keypoint[i] : 0;
        }
        face.score = result.score;
    }
    return count;
}

face_embedder_t *face_embedder_new(void)
{
    return new face_embedder();
}

size_t face_embedding_len(face_embedder_t *embedder)
{
    return embedder->model.get_feat_len();
}

esp_err_t face_embed(face_embedder_t *embedder,
                     const face_image_t *image,
                     const int16_t keypoints[FACE_KEYPOINTS],
                     float *out,
                     size_t len)
{
    std::vector<int> landmarks(keypoints, keypoints + FACE_KEYPOINTS);
    dl::TensorBase *feat = embedder->model.run(to_img(image), landmarks);
    if (!feat || static_cast<size_t>(feat->get_size()) != len) {
        return ESP_ERR_INVALID_SIZE;
    }
    std::memcpy(out, feat->data, len * sizeof(float));
    return ESP_OK;
}
