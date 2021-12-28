CREATE TABLE api_keys (
  id INTEGER AUTO_INCREMENT PRIMARY KEY,
  api_key VARCHAR(255) NOT NULL,
  UNIQUE KEY `KEY_UNIQUE` (`api_key`)
);
CREATE TABLE `nodes` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `node_id_external` varchar(255) NOT NULL,
  `fk_api_key_id` int(11) NOT NULL,
  `monitoring_enabled` tinyint(4) NOT NULL DEFAULT '0',
  `last_checkin_timestamp` datetime NOT NULL,
  `notification_email_list` varchar(255) DEFAULT NULL,
  `offline_notification_sent` tinyint(4) NOT NULL DEFAULT '0',
  PRIMARY KEY (`id`),
  UNIQUE KEY `nodes_UN` (`node_id_external`,`fk_api_key_id`),
  KEY `nodes_node_id_external_IDX` (`node_id_external`) USING BTREE
);
