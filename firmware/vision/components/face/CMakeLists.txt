idf_component_register(
    SRCS "face.cpp"
    INCLUDE_DIRS "include"
    REQUIRES esp32-camera
    PRIV_REQUIRES human_face_detect human_face_recognition esp-dl)
